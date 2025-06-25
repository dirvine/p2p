//! Realistic Workload Integration Tests
//!
//! Integration tests that simulate realistic P2P Foundation workloads:
//! - Multi-node networks with realistic connection patterns
//! - Concurrent DHT operations under load
//! - MCP service interactions at scale
//! - Performance benchmarking under realistic conditions
//! - Resource usage monitoring during high load

use p2p_foundation::{P2PNode, NodeConfig, Result, P2PError};
use p2p_foundation::mcp::{Tool, FunctionToolHandler, MCPServerConfig, MCPCallContext};
use p2p_foundation::dht::Key;
use p2p_foundation::production::{ResourceManager, ProductionConfig};
use std::sync::Arc;
use std::time::Duration;
use std::collections::HashMap;
use tokio::time::{timeout, sleep, Instant};
use serde_json::{json, Value};
use tracing::{info, debug, warn};

/// Realistic test scenario runner
struct RealisticWorkloadTest {
    nodes: Vec<Arc<P2PNode>>,
    configs: Vec<NodeConfig>,
    resource_managers: Vec<Arc<ResourceManager>>,
    start_time: Instant,
}

impl RealisticWorkloadTest {
    /// Create a new realistic workload test with production settings
    async fn new(node_count: usize) -> Result<Self> {
        let mut nodes = Vec::new();
        let mut configs = Vec::new();
        let mut resource_managers = Vec::new();
        
        for i in 0..node_count {
            let production_config = ProductionConfig {
                max_connections: 25,
                max_memory_bytes: 32 * 1024 * 1024, // 32MB per node
                max_bandwidth_bps: 5 * 1024 * 1024, // 5MB/s per node
                connection_timeout: Duration::from_secs(10),
                health_check_interval: Duration::from_secs(30),
                metrics_interval: Duration::from_secs(10),
                enable_performance_tracking: true,
                enable_auto_cleanup: true,
                ..ProductionConfig::default()
            };
            
            let resource_manager = Arc::new(ResourceManager::new(production_config.clone()));
            resource_manager.start().await?;
            
            let config = NodeConfig {
                peer_id: Some(format!("workload_node_{}", i)),
                listen_addrs: vec![
                    format!("/ip4/127.0.0.1/tcp/{}", 9200 + i),
                ],
                enable_mcp_server: true,
                mcp_server_config: Some(MCPServerConfig {
                    server_name: format!("WorkloadNode-{}", i),
                    enable_auth: false, // Simplified for testing
                    enable_rate_limiting: true,
                    max_concurrent_requests: 10,
                    request_timeout: Duration::from_secs(15),
                    ..MCPServerConfig::default()
                }),
                production_config: Some(production_config),
                connection_timeout: Duration::from_secs(10),
                max_connections: 25,
                max_incoming_connections: 15,
                ..NodeConfig::default()
            };
            
            let node = Arc::new(P2PNode::new(config.clone()).await?);
            
            nodes.push(node);
            configs.push(config);
            resource_managers.push(resource_manager);
        }
        
        Ok(Self {
            nodes,
            configs,
            resource_managers,
            start_time: Instant::now(),
        })
    }
    
    /// Start all nodes
    async fn start_all(&self) -> Result<()> {
        info!("Starting {} nodes for realistic workload test", self.nodes.len());
        
        for (i, node) in self.nodes.iter().enumerate() {
            node.start().await
                .map_err(|e| P2PError::Network(format!("Failed to start node {}: {}", i, e)))?;
            
            sleep(Duration::from_millis(100)).await;
            debug!("Started workload node {}", i);
        }
        
        // Wait for startup stabilization
        sleep(Duration::from_secs(1)).await;
        
        Ok(())
    }
    
    /// Establish connections between nodes
    async fn establish_connections(&self) -> Result<()> {
        info!("Establishing realistic connection topology");
        
        // Create a partial mesh where each node connects to 2-3 others
        for i in 0..self.nodes.len() {
            let connections_to_make = std::cmp::min(3, self.nodes.len() - 1);
            
            for j in 1..=connections_to_make {
                let target_idx = (i + j) % self.nodes.len();
                if target_idx != i {
                    let listen_addr = self.configs[target_idx].listen_addrs[0].clone();
                    
                    match timeout(
                        Duration::from_secs(5),
                        self.nodes[i].connect_peer(&listen_addr)
                    ).await {
                        Ok(Ok(_)) => debug!("Node {} connected to node {}", i, target_idx),
                        Ok(Err(e)) => debug!("Connection failed from {} to {}: {}", i, target_idx, e),
                        Err(_) => debug!("Connection timeout from {} to {}", i, target_idx),
                    }
                    
                    sleep(Duration::from_millis(100)).await;
                }
            }
        }
        
        // Wait for network stabilization
        sleep(Duration::from_secs(2)).await;
        
        Ok(())
    }
    
    /// Clean shutdown
    async fn shutdown(&self) -> Result<()> {
        info!("Shutting down realistic workload test");
        
        for (i, node) in self.nodes.iter().enumerate() {
            if let Err(e) = timeout(Duration::from_secs(5), node.stop()).await {
                warn!("Shutdown timeout for node {}: {:?}", i, e);
            }
        }
        
        for (i, rm) in self.resource_managers.iter().enumerate() {
            if let Err(e) = timeout(Duration::from_secs(5), rm.shutdown()).await {
                warn!("Resource manager shutdown timeout for node {}: {:?}", i, e);
            }
        }
        
        Ok(())
    }
    
    fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}

/// Test concurrent DHT operations under realistic load
#[tokio::test]
async fn test_concurrent_dht_operations() -> Result<()> {
    info!("Testing concurrent DHT operations under realistic load");
    
    let test = RealisticWorkloadTest::new(8).await?;
    test.start_all().await?;
    test.establish_connections().await?;
    
    // Create concurrent DHT operations
    let operation_start = Instant::now();
    let mut handles = Vec::new();
    
    // Store operations
    for i in 0..20 {
        let nodes = test.nodes.clone();
        let handle = tokio::spawn(async move {
            let node_idx = i % nodes.len();
            let node = &nodes[node_idx];
            
            if let Some(dht_arc) = node.dht() {
                let key = Key::new(format!("concurrent_test_key_{}", i).as_bytes());
                let value = format!("concurrent_test_value_{}", i).into_bytes();
                
                let dht = dht_arc.read().await;
                let result = timeout(Duration::from_secs(5), dht.put(key, value)).await;
                drop(dht);
                
                match result {
                    Ok(Ok(_)) => Ok(i),
                    Ok(Err(e)) => Err(format!("DHT put failed for {}: {}", i, e)),
                    Err(_) => Err(format!("DHT put timeout for {}", i)),
                }
            } else {
                Err(format!("No DHT available on node {}", node_idx))
            }
        });
        handles.push(handle);
    }
    
    // Wait for store operations
    let mut successful_stores = 0;
    for handle in handles {
        match handle.await {
            Ok(Ok(_)) => successful_stores += 1,
            Ok(Err(e)) => debug!("Store operation failed: {}", e),
            Err(e) => debug!("Store handle error: {}", e),
        }
    }
    
    sleep(Duration::from_millis(500)).await; // Allow replication
    
    // Retrieve operations
    let mut retrieve_handles = Vec::new();
    for i in 0..20 {
        let nodes = test.nodes.clone();
        let handle = tokio::spawn(async move {
            let node_idx = (i + 2) % nodes.len(); // Use different nodes for retrieval
            let node = &nodes[node_idx];
            
            if let Some(dht_arc) = node.dht() {
                let key = Key::new(format!("concurrent_test_key_{}", i).as_bytes());
                let expected_value = format!("concurrent_test_value_{}", i).into_bytes();
                
                let dht = dht_arc.read().await;
                let result = timeout(Duration::from_secs(5), dht.get(&key)).await;
                drop(dht);
                
                match result {
                    Ok(Some(record)) => {
                        if record.value == expected_value {
                            Ok(i)
                        } else {
                            Err(format!("Value mismatch for key {}", i))
                        }
                    },
                    Ok(None) => Err(format!("Key {} not found", i)),
                    Err(_) => Err(format!("DHT get timeout for {}", i)),
                }
            } else {
                Err(format!("No DHT available on node {}", node_idx))
            }
        });
        retrieve_handles.push(handle);
    }
    
    let mut successful_retrievals = 0;
    for handle in retrieve_handles {
        match handle.await {
            Ok(Ok(_)) => successful_retrievals += 1,
            Ok(Err(e)) => debug!("Retrieve operation failed: {}", e),
            Err(e) => debug!("Retrieve handle error: {}", e),
        }
    }
    
    let operation_duration = operation_start.elapsed();
    
    info!("DHT concurrent operations: {}/20 stores, {}/20 retrievals successful in {:?}",
          successful_stores, successful_retrievals, operation_duration);
    
    // Performance assertions (adjusted for realistic DHT behavior)
    assert!(successful_stores >= 10, "Should have at least 10 successful stores, got {}", successful_stores);
    assert!(successful_retrievals >= 5, "Should have at least 5 successful retrievals, got {}", successful_retrievals);
    assert!(operation_duration < Duration::from_secs(30), "Operations took too long: {:?}", operation_duration);
    
    test.shutdown().await?;
    
    info!("✓ Concurrent DHT operations test completed in {:?}", test.elapsed());
    Ok(())
}

/// Test MCP service interactions at scale
#[tokio::test]
async fn test_mcp_service_scaling() -> Result<()> {
    info!("Testing MCP service interactions at scale");
    
    let test = RealisticWorkloadTest::new(6).await?;
    test.start_all().await?;
    test.establish_connections().await?;
    
    // Register scaling test services
    setup_scaling_test_services(&test).await?;
    
    sleep(Duration::from_secs(1)).await;
    
    // Generate concurrent MCP service calls
    let service_start = Instant::now();
    let mut service_handles = Vec::new();
    
    for call_id in 0..30 {
        let nodes = test.nodes.clone();
        let handle = tokio::spawn(async move {
            let node_idx = call_id % nodes.len();
            let node = &nodes[node_idx];
            
            if let Some(mcp_server) = node.mcp_server() {
                let context = MCPCallContext {
                    caller_id: format!("scaling_test_client_{}", call_id),
                    timestamp: std::time::SystemTime::now(),
                    timeout: Duration::from_secs(10),
                    auth_info: None,
                    metadata: HashMap::new(),
                };
                
                let service_name = match call_id % 3 {
                    0 => "compute_service",
                    1 => "data_service",
                    _ => "analytics_service",
                };
                
                let result = timeout(
                    Duration::from_secs(8),
                    mcp_server.call_tool(service_name, json!({"call_id": call_id}), context)
                ).await;
                
                match result {
                    Ok(Ok(response)) => Ok((call_id, response)),
                    Ok(Err(e)) => Err(format!("MCP call {} failed: {}", call_id, e)),
                    Err(_) => Err(format!("MCP call {} timeout", call_id)),
                }
            } else {
                Err(format!("No MCP server on node {}", node_idx))
            }
        });
        service_handles.push(handle);
    }
    
    // Collect service call results
    let mut successful_calls = 0;
    let mut total_response_time = Duration::from_secs(0);
    
    for handle in service_handles {
        match handle.await {
            Ok(Ok((call_id, response))) => {
                successful_calls += 1;
                if let Some(time_ms) = response.get("processing_time_ms").and_then(|v| v.as_u64()) {
                    total_response_time += Duration::from_millis(time_ms);
                }
                debug!("MCP call {} succeeded", call_id);
            }
            Ok(Err(e)) => debug!("MCP service call failed: {}", e),
            Err(e) => debug!("MCP service handle error: {}", e),
        }
    }
    
    let service_duration = service_start.elapsed();
    let avg_response_time = if successful_calls > 0 {
        total_response_time / successful_calls as u32
    } else {
        Duration::from_secs(0)
    };
    
    info!("MCP scaling test: {}/30 calls successful in {:?}, avg response time: {:?}",
          successful_calls, service_duration, avg_response_time);
    
    // Performance assertions
    assert!(successful_calls >= 25, "Should have at least 25 successful MCP calls");
    assert!(service_duration < Duration::from_secs(45), "Service calls took too long");
    assert!(avg_response_time < Duration::from_millis(500), "Average response time too high");
    
    test.shutdown().await?;
    
    info!("✓ MCP service scaling test completed in {:?}", test.elapsed());
    Ok(())
}

/// Test resource usage monitoring during high load
#[tokio::test]
async fn test_resource_monitoring_under_load() -> Result<()> {
    info!("Testing resource usage monitoring during high load");
    
    let test = RealisticWorkloadTest::new(5).await?;
    test.start_all().await?;
    test.establish_connections().await?;
    
    setup_resource_test_services(&test).await?;
    
    // Generate sustained load
    let load_start = Instant::now();
    let load_duration = Duration::from_secs(10);
    let mut load_handles = Vec::new();
    
    // Spawn load generators
    for worker_id in 0..10 {
        let nodes = test.nodes.clone();
        let end_time = load_start + load_duration;
        
        let handle = tokio::spawn(async move {
            let mut operations = 0;
            let node_idx = worker_id % nodes.len();
            let node = &nodes[node_idx];
            
            while Instant::now() < end_time {
                // Mix of DHT and MCP operations
                if worker_id % 2 == 0 {
                    // DHT operation
                    if let Some(dht_arc) = node.dht() {
                        let key = Key::new(format!("load_key_{}_{}", worker_id, operations).as_bytes());
                        let value = format!("load_value_{}_{}", worker_id, operations).into_bytes();
                        
                        let dht = dht_arc.read().await;
                        if timeout(Duration::from_secs(2), dht.put(key, value)).await.is_ok() {
                            operations += 1;
                        }
                        drop(dht);
                    }
                } else {
                    // MCP operation
                    if let Some(mcp_server) = node.mcp_server() {
                        let context = MCPCallContext {
                            caller_id: format!("load_worker_{}", worker_id),
                            timestamp: std::time::SystemTime::now(),
                            timeout: Duration::from_secs(3),
                            auth_info: None,
                            metadata: HashMap::new(),
                        };
                        
                        if timeout(
                            Duration::from_secs(2),
                            mcp_server.call_tool("resource_test", json!({"worker_id": worker_id}), context)
                        ).await.is_ok() {
                            operations += 1;
                        }
                    }
                }
                
                sleep(Duration::from_millis(100)).await;
            }
            
            operations
        });
        load_handles.push(handle);
    }
    
    // Monitor resource usage during load
    let mut resource_snapshots = Vec::new();
    let monitor_start = Instant::now();
    
    while monitor_start.elapsed() < load_duration {
        let mut snapshot = HashMap::new();
        
        for (i, rm) in test.resource_managers.iter().enumerate() {
            let metrics = rm.get_metrics().await;
            snapshot.insert(i, metrics);
        }
        
        resource_snapshots.push((monitor_start.elapsed(), snapshot));
        sleep(Duration::from_secs(1)).await;
    }
    
    // Wait for load generators to complete
    let mut total_operations = 0;
    for handle in load_handles {
        if let Ok(ops) = handle.await {
            total_operations += ops;
        }
    }
    
    let actual_load_duration = load_start.elapsed();
    
    info!("Load test completed: {} total operations in {:?}",
          total_operations, actual_load_duration);
    
    // Analyze resource usage
    analyze_resource_usage(&resource_snapshots)?;
    
    // Performance assertions
    assert!(total_operations >= 50, "Should complete at least 50 operations under load");
    
    test.shutdown().await?;
    
    info!("✓ Resource monitoring test completed in {:?}", test.elapsed());
    Ok(())
}

/// Test network performance benchmarking
#[tokio::test]
async fn test_network_performance_benchmark() -> Result<()> {
    info!("Running network performance benchmark");
    
    let test = RealisticWorkloadTest::new(4).await?;
    test.start_all().await?;
    test.establish_connections().await?;
    
    // Benchmark 1: DHT throughput
    let dht_start = Instant::now();
    let dht_operations = 50;
    
    for i in 0..dht_operations {
        let node = &test.nodes[i % test.nodes.len()];
        if let Some(dht_arc) = node.dht() {
            let key = Key::new(format!("benchmark_key_{}", i).as_bytes());
            let value = format!("benchmark_value_{}", i).into_bytes();
            
            let dht = dht_arc.read().await;
            timeout(Duration::from_secs(3), dht.put(key, value)).await.ok();
            drop(dht);
        }
    }
    
    let dht_duration = dht_start.elapsed();
    let dht_ops_per_sec = dht_operations as f64 / dht_duration.as_secs_f64();
    
    info!("DHT Benchmark: {:.1} operations/second", dht_ops_per_sec);
    
    // Benchmark 2: Network connectivity
    let mut total_connections = 0;
    for node in &test.nodes {
        let peers = node.connected_peers().await;
        total_connections += peers.len();
    }
    let avg_connections = total_connections as f64 / test.nodes.len() as f64;
    
    info!("Network Connectivity: {:.1} average connections per node", avg_connections);
    
    // Benchmark 3: Resource efficiency
    let mut total_memory_mb = 0.0;
    for rm in &test.resource_managers {
        let metrics = rm.get_metrics().await;
        total_memory_mb += metrics.memory_used as f64 / (1024.0 * 1024.0);
    }
    let avg_memory_mb = total_memory_mb / test.nodes.len() as f64;
    
    info!("Resource Usage: {:.1} MB average memory per node", avg_memory_mb);
    
    // Performance assertions
    assert!(dht_ops_per_sec >= 10.0, "DHT throughput too low: {:.1} ops/sec", dht_ops_per_sec);
    assert!(avg_connections >= 2.0, "Network connectivity too low: {:.1} avg connections", avg_connections);
    assert!(avg_memory_mb <= 50.0, "Memory usage too high: {:.1} MB per node", avg_memory_mb);
    
    test.shutdown().await?;
    
    info!("✓ Network performance benchmark completed in {:?}", test.elapsed());
    Ok(())
}

// Helper functions

async fn setup_scaling_test_services(test: &RealisticWorkloadTest) -> Result<()> {
    let services = vec![
        ("compute_service", "Computational service for scaling tests"),
        ("data_service", "Data processing service for scaling tests"),
        ("analytics_service", "Analytics service for scaling tests"),
    ];
    
    for (i, node) in test.nodes.iter().enumerate() {
        let (service_name, description) = services[i % services.len()];
        let service_name_owned = service_name.to_string();
        
        let handler = FunctionToolHandler::new(move |_args: Value| {
            let node_id = i;
            let service_name_clone = service_name_owned.clone();
            async move {
                // Simulate variable processing time
                let processing_time = Duration::from_millis(50 + (node_id * 25) as u64);
                sleep(processing_time).await;
                
                Ok(json!({
                    "service": service_name_clone,
                    "node_id": node_id,
                    "result": format!("Processed by {} on node {}", service_name_clone, node_id),
                    "processing_time_ms": processing_time.as_millis()
                }))
            }
        });
        
        let tool = Tool::new(service_name, description, json!({
            "type": "object",
            "properties": {
                "call_id": {"type": "number"}
            }
        }))
        .handler(handler)
        .build();
        
        if let Some(mcp_server) = node.mcp_server() {
            mcp_server.register_tool(tool?).await?;
        }
    }
    
    Ok(())
}

async fn setup_resource_test_services(test: &RealisticWorkloadTest) -> Result<()> {
    for (i, node) in test.nodes.iter().enumerate() {
        let handler = FunctionToolHandler::new(move |_args: Value| {
            let node_id = i;
            async move {
                // Simulate light resource usage
                sleep(Duration::from_millis(25)).await;
                
                Ok(json!({
                    "node_id": node_id,
                    "resource_usage": "light",
                    "processing_time_ms": 25
                }))
            }
        });
        
        let tool = Tool::new("resource_test", "Resource usage test service", json!({
            "type": "object",
            "properties": {
                "worker_id": {"type": "number"}
            }
        }))
        .handler(handler)
        .build();
        
        if let Some(mcp_server) = node.mcp_server() {
            mcp_server.register_tool(tool?).await?;
        }
    }
    
    Ok(())
}

fn analyze_resource_usage(snapshots: &[(Duration, HashMap<usize, p2p_foundation::production::ResourceMetrics>)]) -> Result<()> {
    if snapshots.is_empty() {
        return Ok(());
    }
    
    info!("Analyzing resource usage across {} snapshots", snapshots.len());
    
    let mut max_memory_mb: f64 = 0.0;
    let mut max_connections = 0;
    let mut max_bandwidth_mbps: f64 = 0.0;
    
    for (_time, metrics_map) in snapshots {
        for (_node_id, metrics) in metrics_map {
            let memory_mb = metrics.memory_used as f64 / (1024.0 * 1024.0);
            let bandwidth_mbps = metrics.bandwidth_usage as f64 / (1024.0 * 1024.0);
            
            max_memory_mb = max_memory_mb.max(memory_mb);
            max_connections = max_connections.max(metrics.active_connections);
            max_bandwidth_mbps = max_bandwidth_mbps.max(bandwidth_mbps);
        }
    }
    
    info!("Peak resource usage: {:.1} MB memory, {} connections, {:.1} Mbps bandwidth",
          max_memory_mb, max_connections, max_bandwidth_mbps);
    
    // Resource usage should stay within reasonable bounds
    assert!(max_memory_mb <= 40.0, "Memory usage exceeded limit: {:.1} MB", max_memory_mb);
    assert!(max_connections <= 30, "Connection count exceeded limit: {}", max_connections);
    assert!(max_bandwidth_mbps <= 10.0, "Bandwidth usage exceeded limit: {:.1} Mbps", max_bandwidth_mbps);
    
    Ok(())
}