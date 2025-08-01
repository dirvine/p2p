# Task 012: Comprehensive Integration Testing

## Overview
Create integration tests that validate the entire system works correctly with all the new error handling, monitoring, and security features. Focus on real-world scenarios and failure modes.

## Acceptance Criteria
- [ ] End-to-end tests for all major flows
- [ ] Failure injection tests
- [ ] Performance regression tests
- [ ] Multi-node network tests
- [ ] All tests automated in CI

## Technical Details

### 1. Test Infrastructure Setup

Location: `crates/p2p-core/tests/integration/`

```rust
// tests/integration/test_harness.rs
use p2p_core::*;
use tokio::time::{sleep, Duration};

pub struct TestNetwork {
    nodes: Vec<TestNode>,
    chaos: ChaosController,
}

pub struct TestNode {
    pub id: String,
    pub network: Network,
    pub dht: DhtClient,
    pub identity: NodeIdentity,
    pub metrics_port: u16,
}

pub struct ChaosController {
    network_delays: HashMap<(String, String), Duration>,
    dropped_connections: HashSet<(String, String)>,
    corrupted_messages: HashSet<String>,
}

impl TestNetwork {
    pub async fn new(node_count: usize) -> Result<Self> {
        let mut nodes = Vec::new();
        
        for i in 0..node_count {
            let config = Config {
                node_name: format!("test-node-{}", i),
                listen_port: 30000 + i as u16,
                metrics_port: 40000 + i as u16,
                ..Default::default()
            };
            
            let identity = NodeIdentity::generate().await?;
            let network = Network::new(config.clone(), identity.clone()).await?;
            let dht = DhtClient::new(config.clone(), network.clone()).await?;
            
            nodes.push(TestNode {
                id: format!("node-{}", i),
                network,
                dht,
                identity,
                metrics_port: config.metrics_port,
            });
        }
        
        Ok(Self {
            nodes,
            chaos: ChaosController::default(),
        })
    }
    
    pub async fn connect_all(&mut self) -> Result<()> {
        // Create full mesh network
        for i in 0..self.nodes.len() {
            for j in i+1..self.nodes.len() {
                let addr = self.nodes[j].network.local_address();
                self.nodes[i].network.connect(&addr).await?;
            }
        }
        Ok(())
    }
}
```

### 2. End-to-End Flow Tests

```rust
// tests/integration/e2e_test.rs
#[tokio::test(flavor = "multi_thread")]
async fn test_full_identity_flow() {
    let mut network = TestNetwork::new(5).await.unwrap();
    network.connect_all().await.unwrap();
    
    // Register identity on node 0
    let identity = network.nodes[0].identity.clone();
    network.nodes[0].dht
        .store_identity(&identity)
        .await
        .unwrap();
    
    // Wait for propagation
    sleep(Duration::from_secs(2)).await;
    
    // Resolve from different node
    let resolved = network.nodes[3].dht
        .resolve_identity(&identity.three_words)
        .await
        .unwrap();
    
    assert_eq!(resolved.unwrap().public_key, identity.public_key);
}

#[tokio::test]
async fn test_network_partition_recovery() {
    let mut network = TestNetwork::new(6).await.unwrap();
    network.connect_all().await.unwrap();
    
    // Store data
    let key = b"test-key";
    let value = b"test-value";
    network.nodes[0].dht.store(key, value).await.unwrap();
    
    // Create partition (0,1,2) | (3,4,5)
    network.chaos.partition_network(vec![0,1,2], vec![3,4,5]);
    
    // Try to retrieve from partitioned node
    let result = network.nodes[4].dht.get(key).await;
    assert!(result.is_err() || result.unwrap().is_none());
    
    // Heal partition
    network.chaos.heal_partition();
    sleep(Duration::from_secs(5)).await;
    
    // Should now work
    let result = network.nodes[4].dht.get(key).await.unwrap();
    assert_eq!(result.unwrap(), value);
}
```

### 3. Error Handling Tests

```rust
// tests/integration/error_handling_test.rs
#[tokio::test]
async fn test_no_panics_under_stress() {
    let network = TestNetwork::new(10).await.unwrap();
    
    // Spawn tasks that try to trigger panics
    let handles: Vec<_> = (0..100).map(|i| {
        let node = network.nodes[i % 10].clone();
        tokio::spawn(async move {
            for _ in 0..1000 {
                // Try operations that previously could panic
                let _ = node.network.connect("invalid:address").await;
                let _ = node.dht.get(b"nonexistent").await;
                let _ = node.identity.verify_signature(b"bad", b"sig");
                
                // Random sleep to create chaos
                sleep(Duration::from_millis(rand::random::<u64>() % 10)).await;
            }
        })
    }).collect();
    
    // All tasks should complete without panic
    for handle in handles {
        handle.await.unwrap();
    }
}

#[tokio::test]
async fn test_connection_exhaustion_handling() {
    let mut network = TestNetwork::new(2).await.unwrap();
    let target = network.nodes[1].network.local_address();
    
    // Try to create more connections than allowed
    let mut handles = vec![];
    for _ in 0..1000 {
        let net = network.nodes[0].network.clone();
        let addr = target.clone();
        handles.push(tokio::spawn(async move {
            net.connect(&addr).await
        }));
    }
    
    let mut success_count = 0;
    let mut error_count = 0;
    
    for handle in handles {
        match handle.await.unwrap() {
            Ok(_) => success_count += 1,
            Err(_) => error_count += 1,
        }
    }
    
    // Should handle gracefully with appropriate errors
    assert!(error_count > 0);
    assert!(success_count <= MAX_CONNECTIONS);
}
```

### 4. Monitoring Integration Tests

```rust
// tests/integration/monitoring_test.rs
#[tokio::test]
async fn test_metrics_collection() {
    let network = TestNetwork::new(3).await.unwrap();
    
    // Perform operations
    network.nodes[0].network
        .connect(&network.nodes[1].network.local_address())
        .await
        .unwrap();
    
    network.nodes[0].dht
        .store(b"key", b"value")
        .await
        .unwrap();
    
    // Check metrics
    let metrics = fetch_metrics(network.nodes[0].metrics_port).await;
    
    assert!(metrics.contains("p2p_connections_total"));
    assert!(metrics.contains("p2p_dht_operations_total"));
    
    // Verify counters increased
    let conn_count = parse_metric(&metrics, "p2p_connections_total");
    assert!(conn_count > 0.0);
}

#[tokio::test]
async fn test_health_checks() {
    let network = TestNetwork::new(1).await.unwrap();
    
    // Check liveness
    let response = reqwest::get(&format!(
        "http://localhost:{}/health/live",
        network.nodes[0].metrics_port
    ))
    .await
    .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let health: HealthResponse = response.json().await.unwrap();
    assert_eq!(health.status, "alive");
    
    // Check readiness (no peers, should be not ready)
    let response = reqwest::get(&format!(
        "http://localhost:{}/health/ready",
        network.nodes[0].metrics_port
    ))
    .await
    .unwrap();
    
    assert_eq!(response.status(), 503);
}
```

### 5. Performance Regression Tests

```rust
// tests/integration/performance_test.rs
#[tokio::test]
async fn test_error_handling_performance() {
    let network = TestNetwork::new(2).await.unwrap();
    network.connect_all().await.unwrap();
    
    // Baseline performance
    let start = Instant::now();
    for _ in 0..10000 {
        network.nodes[0].dht.get(b"test").await.ok();
    }
    let baseline = start.elapsed();
    
    // Performance with error handling
    let start = Instant::now();
    for _ in 0..10000 {
        // Force error path
        network.nodes[0].dht.get(b"nonexistent").await.ok();
    }
    let with_errors = start.elapsed();
    
    // Should not be more than 10% slower
    assert!(
        with_errors.as_secs_f64() < baseline.as_secs_f64() * 1.1,
        "Error handling overhead too high: {:?} vs {:?}",
        with_errors,
        baseline
    );
}
```

### 6. Chaos Testing

```rust
// tests/integration/chaos_test.rs
impl ChaosController {
    pub fn inject_network_delay(&mut self, from: usize, to: usize, delay: Duration) {
        self.network_delays.insert((from.to_string(), to.to_string()), delay);
    }
    
    pub fn drop_connections(&mut self, from: usize, to: usize) {
        self.dropped_connections.insert((from.to_string(), to.to_string()));
    }
    
    pub fn corrupt_messages(&mut self, node: usize) {
        self.corrupted_messages.insert(node.to_string());
    }
}

#[tokio::test]
async fn test_network_chaos_resilience() {
    let mut network = TestNetwork::new(5).await.unwrap();
    network.connect_all().await.unwrap();
    
    // Inject various failures
    network.chaos.inject_network_delay(0, 1, Duration::from_secs(5));
    network.chaos.drop_connections(2, 3);
    network.chaos.corrupt_messages(4);
    
    // System should continue operating
    let key = b"chaos-test";
    let value = b"chaos-value";
    
    // Store with retries
    let mut stored = false;
    for _ in 0..5 {
        if network.nodes[0].dht.store(key, value).await.is_ok() {
            stored = true;
            break;
        }
        sleep(Duration::from_secs(1)).await;
    }
    assert!(stored);
    
    // Eventually consistent retrieval
    let mut retrieved = false;
    for _ in 0..10 {
        for node in &network.nodes {
            if let Ok(Some(v)) = node.dht.get(key).await {
                if v == value {
                    retrieved = true;
                    break;
                }
            }
        }
        if retrieved { break; }
        sleep(Duration::from_secs(1)).await;
    }
    assert!(retrieved);
}
```

## Testing Requirements
- All tests must be deterministic
- Tests should clean up resources
- Parallel test execution support
- CI integration with test reports
- Coverage reports generated

## Dependencies
- Previous: All implementation tasks
- External: tokio-test, reqwest, proptest

## Time Estimate
- Test infrastructure: 4 hours
- E2E tests: 6 hours
- Chaos tests: 4 hours
- Performance tests: 3 hours
- CI integration: 2 hours
- Total: 19 hours

## Definition of Done
- [ ] All test categories implemented
- [ ] Tests run in CI automatically
- [ ] No flaky tests
- [ ] Coverage > 80%
- [ ] Performance benchmarks tracked