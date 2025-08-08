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

//! HTTP endpoints for health checks

use super::HealthManager;
use crate::health::metrics::PrometheusExporter;
use crate::{P2PError, Result};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpListener;
use tracing::{error, info};

/// Health check HTTP endpoints
pub struct HealthEndpoints {
    health_manager: Arc<HealthManager>,
    prometheus_exporter: Arc<PrometheusExporter>,
}

impl HealthEndpoints {
    /// Create new health endpoints
    pub fn new(health_manager: Arc<HealthManager>) -> Self {
        Self {
            health_manager: health_manager.clone(),
            prometheus_exporter: Arc::new(PrometheusExporter::new(health_manager)),
        }
    }

    /// Build the router with all health endpoints
    pub fn router(self) -> Router {
        Router::new()
            .route("/health", get(health_handler))
            .route("/ready", get(ready_handler))
            .route("/metrics", get(metrics_handler))
            .route("/debug/vars", get(debug_handler))
            .with_state(Arc::new(self))
    }

    /// Serve the health endpoints on the specified address
    pub async fn serve(self, addr: std::net::SocketAddr) -> Result<()> {
        let router = self.router();

        info!("Starting health server on {}", addr);

        let listener = TcpListener::bind(addr).await.map_err(|e| {
            P2PError::Network(crate::error::NetworkError::BindError(
                format!("Failed to bind health server: {}", e).into(),
            ))
        })?;

        axum::serve(listener, router).await.map_err(|e| {
            P2PError::Network(crate::error::NetworkError::ProtocolError(
                format!("Health server error: {}", e).into(),
            ))
        })?;

        Ok(())
    }
}

/// Health server that runs the HTTP endpoints
pub struct HealthServer {
    endpoints: HealthEndpoints,
    addr: std::net::SocketAddr,
    shutdown_signal: tokio::sync::oneshot::Receiver<()>,
}

impl HealthServer {
    /// Create a new health server
    pub fn new(
        health_manager: Arc<HealthManager>,
        addr: std::net::SocketAddr,
    ) -> (Self, tokio::sync::oneshot::Sender<()>) {
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        let server = Self {
            endpoints: HealthEndpoints::new(health_manager),
            addr,
            shutdown_signal: shutdown_rx,
        };

        (server, shutdown_tx)
    }

    /// Run the health server
    pub async fn run(self) -> Result<()> {
        let router = self.endpoints.router();

        info!("Starting health server on {}", self.addr);

        let listener = TcpListener::bind(self.addr).await.map_err(|e| {
            P2PError::Network(crate::error::NetworkError::BindError(
                format!("Failed to bind health server: {}", e).into(),
            ))
        })?;

        let server = axum::serve(listener, router).with_graceful_shutdown(async move {
            let _ = self.shutdown_signal.await;
            info!("Health server shutting down");
        });

        server.await.map_err(|e| {
            P2PError::Network(crate::error::NetworkError::ProtocolError(
                format!("Health server error: {}", e).into(),
            ))
        })?;

        Ok(())
    }
}

/// Handler for /health endpoint (liveness check)
async fn health_handler(
    State(_endpoints): State<Arc<HealthEndpoints>>,
) -> std::result::Result<impl IntoResponse, HealthError> {
    let start = Instant::now();

    // Basic liveness check - if we can respond, we're alive
    let response = serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    let latency = start.elapsed();
    if latency.as_millis() > 100 {
        error!(
            "Health check took {}ms (> 100ms threshold)",
            latency.as_millis()
        );
    }

    Ok(Json(response))
}

/// Handler for /ready endpoint (readiness check)
async fn ready_handler(
    State(endpoints): State<Arc<HealthEndpoints>>,
) -> std::result::Result<impl IntoResponse, HealthError> {
    let start = Instant::now();

    // Get full health status to determine readiness
    let health = endpoints
        .health_manager
        .get_health()
        .await
        .map_err(|e| HealthError::Internal(e.to_string()))?;

    let latency = start.elapsed();
    if latency.as_millis() > 100 {
        error!(
            "Ready check took {}ms (> 100ms threshold)",
            latency.as_millis()
        );
    }

    if health.is_ready() {
        Ok((StatusCode::OK, Json(health)))
    } else {
        Ok((StatusCode::SERVICE_UNAVAILABLE, Json(health)))
    }
}

/// Handler for /metrics endpoint (Prometheus format)
async fn metrics_handler(
    State(endpoints): State<Arc<HealthEndpoints>>,
) -> std::result::Result<impl IntoResponse, HealthError> {
    let start = Instant::now();

    let metrics = endpoints
        .prometheus_exporter
        .export()
        .await
        .map_err(|e| HealthError::Internal(e.to_string()))?;

    let latency = start.elapsed();
    if latency.as_millis() > 100 {
        error!(
            "Metrics export took {}ms (> 100ms threshold)",
            latency.as_millis()
        );
    }

    Ok((
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4",
        )],
        metrics,
    ))
}

/// Handler for /debug/vars endpoint
async fn debug_handler(
    State(endpoints): State<Arc<HealthEndpoints>>,
) -> std::result::Result<impl IntoResponse, HealthError> {
    let start = Instant::now();

    let debug_info = endpoints
        .health_manager
        .get_debug_info()
        .await
        .map_err(|e| HealthError::Internal(e.to_string()))?;

    let latency = start.elapsed();
    if latency.as_millis() > 100 {
        error!(
            "Debug info took {}ms (> 100ms threshold)",
            latency.as_millis()
        );
    }

    Ok(Json(debug_info))
}

/// Error type for health endpoints
#[derive(Debug)]
enum HealthError {
    Internal(String),
}

impl IntoResponse for HealthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            HealthError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = serde_json::json!({
            "error": message,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::{DebugInfo, HealthManager, HealthResponse};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::util::ServiceExt;

    async fn create_test_app() -> Router {
        let health_manager = Arc::new(HealthManager::new("1.0.0".to_string()));
        let endpoints = HealthEndpoints::new(health_manager);
        endpoints.router()
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let app = create_test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["status"], "healthy");
        assert!(json["timestamp"].is_string());
    }

    #[tokio::test]
    async fn test_ready_endpoint() {
        let app = create_test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let health: HealthResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(health.status, "healthy");
        assert!(health.version.len() > 0);
    }

    #[tokio::test]
    async fn test_metrics_endpoint() {
        let app = create_test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "text/plain; version=0.0.4"
        );

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let metrics = String::from_utf8(body.to_vec()).unwrap();

        // Check for standard Prometheus format
        assert!(metrics.contains("# HELP"));
        assert!(metrics.contains("# TYPE"));
    }

    #[tokio::test]
    async fn test_debug_endpoint() {
        let app = create_test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/debug/vars")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let debug: DebugInfo = serde_json::from_slice(&body).unwrap();

        assert!(!debug.system.os.is_empty());
        assert!(!debug.system.arch.is_empty());
        assert!(debug.system.cpu_count > 0);
    }

    #[tokio::test]
    async fn test_not_found() {
        let app = create_test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/invalid")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_health_server_lifecycle() {
        let health_manager = Arc::new(HealthManager::new("1.0.0".to_string()));
        let addr = "127.0.0.1:0".parse().unwrap();

        let (server, shutdown_tx) = HealthServer::new(health_manager, addr);

        // Start server in background
        let server_handle = tokio::spawn(async move { server.run().await });

        // Give server time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Shutdown server
        shutdown_tx.send(()).unwrap();

        // Wait for server to stop
        let result = server_handle.await.unwrap();
        assert!(result.is_ok());
    }
}
