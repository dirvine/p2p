# Task 011: Monitoring and Observability Infrastructure

## Overview
Implement comprehensive monitoring with Prometheus metrics, health check endpoints, and structured logging to enable production observability.

## Acceptance Criteria
- [ ] Prometheus metrics endpoint operational
- [ ] Health check endpoints (liveness/readiness)
- [ ] Structured logging with tracing
- [ ] Key metrics identified and implemented
- [ ] Grafana dashboard templates created

## Technical Details

### 1. Metrics Registry Setup

Location: `crates/p2p-core/src/monitoring/mod.rs`

```rust
use prometheus::{
    Registry, Counter, CounterVec, Gauge, GaugeVec, 
    Histogram, HistogramVec, HistogramOpts
};
use once_cell::sync::Lazy;

pub static METRICS: Lazy<P2PMetrics> = Lazy::new(|| {
    P2PMetrics::new().expect("Failed to create metrics")
});

pub struct P2PMetrics {
    registry: Registry,
    
    // Connection metrics
    pub connections_total: CounterVec,
    pub connections_active: GaugeVec,
    pub connection_duration_seconds: HistogramVec,
    pub connection_errors_total: CounterVec,
    
    // DHT metrics
    pub dht_operations_total: CounterVec,
    pub dht_operation_duration_seconds: HistogramVec,
    pub dht_stored_keys: Gauge,
    pub dht_replication_factor: Gauge,
    
    // Network metrics
    pub bytes_sent_total: CounterVec,
    pub bytes_received_total: CounterVec,
    pub messages_sent_total: CounterVec,
    pub messages_received_total: CounterVec,
    
    // Identity metrics
    pub identity_operations_total: CounterVec,
    pub identity_verification_duration_seconds: Histogram,
    
    // System metrics
    pub process_cpu_seconds_total: Counter,
    pub process_resident_memory_bytes: Gauge,
    pub process_open_fds: Gauge,
}

impl P2PMetrics {
    pub fn new() -> Result<Self> {
        let registry = Registry::new();
        
        let metrics = Self {
            registry: registry.clone(),
            
            connections_total: CounterVec::new(
                prometheus::Opts::new(
                    "p2p_connections_total",
                    "Total number of connection attempts"
                ),
                &["status", "direction"]
            )?,
            
            connections_active: GaugeVec::new(
                prometheus::Opts::new(
                    "p2p_connections_active",
                    "Currently active connections"
                ),
                &["peer_id", "transport"]
            )?,
            
            connection_duration_seconds: HistogramVec::new(
                HistogramOpts::new(
                    "p2p_connection_duration_seconds",
                    "Connection duration in seconds"
                ).buckets(vec![0.1, 0.5, 1.0, 5.0, 10.0, 30.0, 60.0]),
                &["peer_id"]
            )?,
            
            // ... initialize other metrics
        };
        
        // Register all metrics
        registry.register(Box::new(metrics.connections_total.clone()))?;
        registry.register(Box::new(metrics.connections_active.clone()))?;
        // ... register others
        
        Ok(metrics)
    }
    
    pub fn render(&self) -> String {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }
}
```

### 2. Metrics HTTP Server

```rust
use axum::{Router, response::IntoResponse, http::StatusCode};

pub async fn start_metrics_server(port: u16) -> Result<()> {
    let app = Router::new()
        .route("/metrics", axum::routing::get(metrics_handler))
        .route("/health/live", axum::routing::get(liveness_handler))
        .route("/health/ready", axum::routing::get(readiness_handler));
    
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    
    Ok(())
}

async fn metrics_handler() -> impl IntoResponse {
    (StatusCode::OK, METRICS.render())
}
```

### 3. Health Check Implementation

```rust
use serde::Serialize;

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    timestamp: DateTime<Utc>,
    version: String,
    uptime_seconds: u64,
    checks: HashMap<String, ComponentHealth>,
}

#[derive(Serialize)]
struct ComponentHealth {
    status: String,
    message: Option<String>,
}

static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

async fn liveness_handler() -> impl IntoResponse {
    // Basic liveness - is the process running?
    let response = HealthResponse {
        status: "alive".to_string(),
        timestamp: Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: START_TIME.elapsed().as_secs(),
        checks: HashMap::new(),
    };
    
    (StatusCode::OK, Json(response))
}

async fn readiness_handler(State(app_state): State<AppState>) -> impl IntoResponse {
    let mut checks = HashMap::new();
    let mut all_healthy = true;
    
    // Check network connectivity
    let network_health = if app_state.network.peer_count() > 0 {
        ComponentHealth {
            status: "healthy".to_string(),
            message: Some(format!("{} peers connected", app_state.network.peer_count())),
        }
    } else {
        all_healthy = false;
        ComponentHealth {
            status: "unhealthy".to_string(),
            message: Some("No peers connected".to_string()),
        }
    };
    checks.insert("network".to_string(), network_health);
    
    // Check DHT
    let dht_health = match app_state.dht.health_check().await {
        Ok(()) => ComponentHealth {
            status: "healthy".to_string(),
            message: None,
        },
        Err(e) => {
            all_healthy = false;
            ComponentHealth {
                status: "unhealthy".to_string(),
                message: Some(e.to_string()),
            }
        }
    };
    checks.insert("dht".to_string(), dht_health);
    
    let response = HealthResponse {
        status: if all_healthy { "ready" } else { "not_ready" }.to_string(),
        timestamp: Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: START_TIME.elapsed().as_secs(),
        checks,
    };
    
    let status = if all_healthy { 
        StatusCode::OK 
    } else { 
        StatusCode::SERVICE_UNAVAILABLE 
    };
    
    (status, Json(response))
}
```

### 4. Structured Logging Setup

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_logging(config: &Config) -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));
    
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .json(); // JSON format for production
    
    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer);
    
    // Optional: OpenTelemetry integration
    if config.otel_enabled {
        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(&config.otel_endpoint)
            )
            .with_trace_config(
                trace::config()
                    .with_sampler(Sampler::AlwaysOn)
                    .with_resource(Resource::new(vec![
                        KeyValue::new("service.name", "p2p-foundation"),
                        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                    ]))
            )
            .install_batch(opentelemetry::runtime::Tokio)?;
        
        let otel_layer = tracing_opentelemetry::layer()
            .with_tracer(tracer);
        
        registry.with(otel_layer).init();
    } else {
        registry.init();
    }
    
    Ok(())
}
```

### 5. Instrumentation Examples

```rust
use tracing::{instrument, info, warn, error};

#[instrument(skip(self, data), fields(peer_id = %peer_id, data_len = data.len()))]
pub async fn send_to_peer(&self, peer_id: &PeerId, data: Vec<u8>) -> Result<()> {
    info!("Sending data to peer");
    
    // Record metrics
    METRICS.messages_sent_total
        .with_label_values(&[&peer_id.to_string()])
        .inc();
    
    METRICS.bytes_sent_total
        .with_label_values(&[&peer_id.to_string()])
        .inc_by(data.len() as u64);
    
    let start = Instant::now();
    
    match self.send_internal(peer_id, data).await {
        Ok(()) => {
            let duration = start.elapsed();
            info!(duration_ms = duration.as_millis(), "Message sent successfully");
            Ok(())
        }
        Err(e) => {
            error!(error = %e, "Failed to send message");
            METRICS.connection_errors_total
                .with_label_values(&["send_failed", &peer_id.to_string()])
                .inc();
            Err(e)
        }
    }
}
```

### 6. Grafana Dashboard Template

Create `monitoring/grafana/p2p-dashboard.json`:
```json
{
  "dashboard": {
    "title": "P2P Foundation Monitoring",
    "panels": [
      {
        "title": "Active Connections",
        "targets": [{
          "expr": "sum(p2p_connections_active)"
        }]
      },
      {
        "title": "Message Rate",
        "targets": [{
          "expr": "rate(p2p_messages_sent_total[5m])"
        }]
      },
      {
        "title": "DHT Operations",
        "targets": [{
          "expr": "rate(p2p_dht_operations_total[5m])"
        }]
      },
      {
        "title": "Error Rate",
        "targets": [{
          "expr": "rate(p2p_connection_errors_total[5m])"
        }]
      }
    ]
  }
}
```

## Testing Requirements
- Metrics endpoint accessibility test
- Health check response validation
- Load test with metrics collection
- Verify no performance impact
- Grafana dashboard validation

## Dependencies
- Previous: All core module tasks
- External: prometheus, axum, tracing

## Time Estimate
- Metrics implementation: 6 hours
- Health checks: 3 hours
- Logging setup: 3 hours
- Dashboard creation: 2 hours
- Testing: 2 hours
- Total: 16 hours

## Definition of Done
- [ ] Metrics endpoint returns Prometheus format
- [ ] Health checks respond correctly
- [ ] Structured logging operational
- [ ] Key metrics being collected
- [ ] Grafana dashboard functional