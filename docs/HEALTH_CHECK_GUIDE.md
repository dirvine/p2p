# Health Check System Guide

## Overview

The P2P Foundation includes a comprehensive health check system that provides:

- **Liveness checks** - Basic confirmation the service is running
- **Readiness checks** - Verification the service can handle traffic
- **Component health** - Individual subsystem status monitoring
- **Prometheus metrics** - Standard metrics export for monitoring
- **Debug information** - Detailed system state for troubleshooting

## Quick Start

### Basic Health Check Setup

```rust
use saorsa_core::health::{HealthManager, NetworkHealthChecker, DhtHealthChecker};
use std::sync::Arc;

// Create health manager
let health_manager = Arc::new(HealthManager::new("1.0.0".to_string()));

// Register component checkers
health_manager.register_checker(
    "network",
    Box::new(NetworkHealthChecker::new(|| async {
        // Get peer count from your network implementation
        Ok(network.peer_count().await?)
    }).with_min_peers(3))
).await;

health_manager.register_checker(
    "dht",
    Box::new(DhtHealthChecker::new(|| async {
        // Get routing table size from DHT
        Ok(dht.routing_table_size().await?)
    }).with_min_nodes(5))
).await;

// Start health server
let addr = "0.0.0.0:8080".parse().unwrap();
let (server, shutdown_tx) = HealthServer::new(health_manager, addr);

tokio::spawn(async move {
    server.run().await.unwrap();
});
```

## HTTP Endpoints

### `/health` - Liveness Check

Basic health check to verify the service is running.

```bash
curl http://localhost:8080/health
```

Response:
```json
{
  "status": "healthy",
  "timestamp": "2024-01-15T10:30:00Z"
}
```

### `/ready` - Readiness Check

Comprehensive check to verify the service is ready to handle traffic.

```bash
curl http://localhost:8080/ready
```

Response:
```json
{
  "status": "healthy",
  "version": "1.0.0",
  "uptime": 3600,
  "checks": {
    "network": {
      "status": "healthy",
      "latency_ms": 15,
      "metadata": {
        "peer_count": 25,
        "min_peers": 3
      }
    },
    "dht": {
      "status": "healthy",
      "latency_ms": 8,
      "metadata": {
        "routing_table_size": 150,
        "min_nodes": 5
      }
    }
  },
  "timestamp": "2024-01-15T10:30:00Z"
}
```

Status codes:
- `200 OK` - Service is ready
- `503 Service Unavailable` - Service is not ready

### `/metrics` - Prometheus Metrics

Exports metrics in Prometheus format for monitoring.

```bash
curl http://localhost:8080/metrics
```

Sample output:
```
# HELP p2p_node_info Node information
# TYPE p2p_node_info gauge
p2p_node_info{version="1.0.0",os="linux",arch="x86_64"} 1

# HELP p2p_uptime_seconds Node uptime in seconds
# TYPE p2p_uptime_seconds counter
p2p_uptime_seconds 3600

# HELP p2p_health_status Health status of components (1=healthy, 0=unhealthy)
# TYPE p2p_health_status gauge
p2p_health_status{component="network"} 1
p2p_health_status{component="dht"} 1

# HELP p2p_healthy_components Number of healthy components
# TYPE p2p_healthy_components gauge
p2p_healthy_components 2
```

### `/debug/vars` - Debug Information

Detailed system and component information for debugging.

```bash
curl http://localhost:8080/debug/vars
```

Response:
```json
{
  "system": {
    "os": "linux",
    "arch": "x86_64",
    "cpu_count": 8,
    "total_memory": 17179869184,
    "available_memory": 8589934592
  },
  "runtime": {
    "rust_version": "0.1.0",
    "thread_count": 10,
    "memory_usage": 104857600,
    "uptime": 3600
  },
  "components": {
    "network": {
      "peer_count": 25,
      "active_connections": 20
    }
  }
}
```

## Built-in Component Checkers

### NetworkHealthChecker

Monitors network connectivity and peer connections.

```rust
let checker = NetworkHealthChecker::new(|| async {
    Ok(network.peer_count().await?)
})
.with_min_peers(3); // Minimum peers for healthy status
```

### DhtHealthChecker

Monitors DHT routing table and availability.

```rust
let checker = DhtHealthChecker::new(|| async {
    Ok(dht.routing_table_size().await?)
})
.with_min_nodes(5); // Minimum nodes for healthy status
```

### StorageHealthChecker

Monitors storage availability and free space.

```rust
let checker = StorageHealthChecker::new("/var/lib/p2p".into())
    .with_min_free_space(100 * 1024 * 1024); // 100MB minimum
```

### ResourceHealthChecker

Monitors system resources (CPU, memory, connections).

```rust
let checker = ResourceHealthChecker::new(resource_manager);
```

### TransportHealthChecker

Monitors transport layer status.

```rust
let checker = TransportHealthChecker::new(|| async {
    Ok(transport.is_listening().await?)
});
```

### PeerHealthChecker

Monitors peer connection counts with thresholds.

```rust
let checker = PeerHealthChecker::new(|| async {
    Ok(network.peer_count().await?)
})
.with_peer_limits(10, 1000); // Min 10, max 1000 peers
```

## Custom Health Checkers

Implement the `ComponentChecker` trait for custom health checks:

```rust
use async_trait::async_trait;
use saorsa_core::health::{ComponentChecker, HealthStatus};
use saorsa_core::Result;

struct CustomChecker {
    threshold: f64,
}

#[async_trait]
impl ComponentChecker for CustomChecker {
    async fn check(&self) -> Result<HealthStatus> {
        let metric = get_custom_metric().await?;
        
        if metric > self.threshold {
            Ok(HealthStatus::Healthy)
        } else if metric > self.threshold * 0.5 {
            Ok(HealthStatus::Degraded)
        } else {
            Ok(HealthStatus::Unhealthy)
        }
    }
    
    async fn debug_info(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "threshold": self.threshold,
            "current_value": get_custom_metric().await.ok()
        }))
    }
}
```

## Integration with Kubernetes

### Liveness Probe

```yaml
livenessProbe:
  httpGet:
    path: /health
    port: 8080
  initialDelaySeconds: 10
  periodSeconds: 10
  timeoutSeconds: 5
```

### Readiness Probe

```yaml
readinessProbe:
  httpGet:
    path: /ready
    port: 8080
  initialDelaySeconds: 30
  periodSeconds: 10
  timeoutSeconds: 5
```

## Prometheus Configuration

```yaml
scrape_configs:
  - job_name: 'p2p-node'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
    scrape_interval: 15s
```

## Performance Considerations

1. **Response Time** - All health checks complete within 100ms
2. **Caching** - Results cached for 100ms to prevent overload
3. **Parallel Checks** - Component checks run concurrently
4. **Timeouts** - Individual checks timeout after 50ms by default

## Graceful Degradation

The health system supports three states:

- **Healthy** - All components functioning normally
- **Degraded** - Some components degraded but service operational
- **Unhealthy** - Critical components failed, service not operational

A degraded service will:
- Continue serving existing connections
- Accept new connections with warnings
- Report degraded status in metrics

An unhealthy service will:
- Reject new connections
- Complete existing requests
- Trigger alerts in monitoring

## Best Practices

1. **Register all critical components** - Ensure comprehensive monitoring
2. **Set appropriate thresholds** - Balance sensitivity with stability
3. **Monitor response times** - Track health check latency
4. **Use composite checkers** - Group related components
5. **Export metrics** - Enable proactive monitoring
6. **Handle degradation gracefully** - Implement fallback behavior

## Troubleshooting

### Health checks timing out

- Check individual component latencies
- Increase timeout duration if needed
- Verify components are responsive

### False positives

- Adjust thresholds based on normal operation
- Add hysteresis to prevent flapping
- Consider using degraded state

### Missing components

- Ensure all checkers are registered
- Verify registration happens before server start
- Check for registration errors

## Example: Complete Setup

```rust
use saorsa_core::health::*;
use saorsa_core::production::{ProductionConfig, ResourceManager};
use std::sync::Arc;

async fn setup_health_monitoring() -> Result<()> {
    // Create resource manager
    let config = ProductionConfig::default();
    let resource_manager = Arc::new(ResourceManager::new(config));
    
    // Create health manager
    let health_manager = Arc::new(HealthManager::new(
        env!("CARGO_PKG_VERSION").to_string()
    ));
    
    // Register all component checkers
    health_manager.register_checker(
        "network",
        Box::new(NetworkHealthChecker::new(|| async {
            // Your network implementation
            Ok(10) // Example: 10 peers
        }).with_min_peers(3))
    ).await;
    
    health_manager.register_checker(
        "dht",
        Box::new(DhtHealthChecker::new(|| async {
            // Your DHT implementation
            Ok(50) // Example: 50 nodes
        }).with_min_nodes(5))
    ).await;
    
    health_manager.register_checker(
        "storage",
        Box::new(StorageHealthChecker::new("/var/lib/p2p".into())
            .with_min_free_space(1024 * 1024 * 1024)) // 1GB
    ).await;
    
    health_manager.register_checker(
        "resources",
        Box::new(ResourceHealthChecker::new(resource_manager.clone()))
    ).await;
    
    health_manager.register_checker(
        "transport",
        Box::new(TransportHealthChecker::new(|| async {
            // Your transport implementation
            Ok(true) // Example: listening
        }))
    ).await;
    
    // Create composite checker for critical components
    let critical_checker = CompositeHealthChecker::new()
        .add_checker("network", Box::new(NetworkHealthChecker::new(|| async { Ok(10) })))
        .add_checker("dht", Box::new(DhtHealthChecker::new(|| async { Ok(50) })));
    
    health_manager.register_checker(
        "critical",
        Box::new(critical_checker)
    ).await;
    
    // Start health server
    let addr = "0.0.0.0:8080".parse().unwrap();
    let (server, _shutdown_tx) = HealthServer::new(health_manager, addr);
    
    server.run().await?;
    
    Ok(())
}
```

This health check system provides comprehensive monitoring capabilities while maintaining high performance and minimal overhead.