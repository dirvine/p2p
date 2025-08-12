// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// MCP server implementation for remote management

use anyhow::{Result, Context};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json;
use super::{MCPRequest, MCPResponse, MCPAuth, error_codes};
use super::handlers::MCPHandlers;

/// MCP server configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MCPConfig {
    /// Listen address
    pub listen_addr: String,
    /// Listen port
    pub port: u16,
    /// Enable TLS
    pub tls_enabled: bool,
    /// TLS certificate path
    pub tls_cert: Option<String>,
    /// TLS key path
    pub tls_key: Option<String>,
    /// Authentication required
    pub auth_required: bool,
    /// API tokens
    pub api_tokens: Vec<String>,
    /// Max concurrent connections
    pub max_connections: usize,
    /// Request timeout (seconds)
    pub request_timeout: u64,
}

impl Default for MCPConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0".to_string(),
            port: 9090,
            tls_enabled: false,
            tls_cert: None,
            tls_key: None,
            auth_required: true,
            api_tokens: vec![],
            max_connections: 100,
            request_timeout: 30,
        }
    }
}

/// MCP server for remote node management
#[derive(Debug, Clone)]
pub struct MCPServer {
    config: Arc<RwLock<MCPConfig>>,
    handlers: Arc<MCPHandlers>,
    active_connections: Arc<RwLock<usize>>,
}

impl MCPServer {
    /// Create a new MCP server
    pub fn new(config: MCPConfig, handlers: MCPHandlers) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            handlers: Arc::new(handlers),
            active_connections: Arc::new(RwLock::new(0)),
        }
    }
    
    /// Start the MCP server
    pub async fn start(&self) -> Result<()> {
        let config = self.config.read().await;
        let addr = format!("{}:{}", config.listen_addr, config.port);
        
        println!("Starting MCP server on {}", addr);
        
        let listener = TcpListener::bind(&addr).await
            .context("Failed to bind MCP server")?;
        
        println!("MCP server listening on {}", addr);
        
        // Accept connections
        loop {
            let (stream, peer_addr) = listener.accept().await?;
            
            // Check connection limit
            let active = *self.active_connections.read().await;
            if active >= config.max_connections {
                println!("Connection limit reached, rejecting {}", peer_addr);
                continue;
            }
            
            // Handle connection in background
            let config = self.config.clone();
            let handlers = self.handlers.clone();
            let active_connections = self.active_connections.clone();
            
            tokio::spawn(async move {
                // Increment connection count
                {
                    let mut count = active_connections.write().await;
                    *count += 1;
                }
                
                // Handle connection
                if let Err(e) = handle_connection(stream, config, handlers).await {
                    eprintln!("Error handling connection from {}: {}", peer_addr, e);
                }
                
                // Decrement connection count
                {
                    let mut count = active_connections.write().await;
                    *count -= 1;
                }
            });
        }
    }
    
    /// Stop the MCP server
    pub async fn stop(&self) -> Result<()> {
        // TODO: Implement graceful shutdown
        Ok(())
    }
    
    /// Get server statistics
    pub async fn get_stats(&self) -> Result<ServerStats> {
        Ok(ServerStats {
            active_connections: *self.active_connections.read().await,
            total_requests: 0, // TODO: Track
            total_errors: 0, // TODO: Track
            uptime_seconds: 0, // TODO: Track
        })
    }
}

/// Server statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct ServerStats {
    pub active_connections: usize,
    pub total_requests: u64,
    pub total_errors: u64,
    pub uptime_seconds: u64,
}

/// Handle a single MCP connection
async fn handle_connection(
    mut stream: TcpStream,
    config: Arc<RwLock<MCPConfig>>,
    handlers: Arc<MCPHandlers>,
) -> Result<()> {
    // Read request
    let mut buffer = vec![0; 65536]; // 64KB buffer
    let timeout = std::time::Duration::from_secs(config.read().await.request_timeout);
    
    let n = tokio::time::timeout(timeout, stream.read(&mut buffer)).await
        .context("Request timeout")?
        .context("Failed to read request")?;
    
    if n == 0 {
        return Ok(()); // Connection closed
    }
    
    // Parse request
    let request_data = &buffer[..n];
    let request: MCPRequest = match serde_json::from_slice(request_data) {
        Ok(req) => req,
        Err(e) => {
            let response = MCPResponse::Error {
                error: format!("Invalid request: {}", e),
                code: error_codes::INVALID_REQUEST,
            };
            send_response(&mut stream, &response).await?;
            return Ok(());
        }
    };
    
    // Check authentication if required
    let auth = if config.read().await.auth_required {
        // Extract auth token from request headers (simplified)
        // In production, use proper HTTP headers or TLS client certificates
        let token = extract_auth_token(request_data);
        
        if let Some(token) = token {
            if config.read().await.api_tokens.contains(&token) {
                Some(MCPAuth::new(token, vec!["*".to_string()]))
            } else {
                let response = MCPResponse::Error {
                    error: "Invalid authentication token".to_string(),
                    code: error_codes::UNAUTHORIZED,
                };
                send_response(&mut stream, &response).await?;
                return Ok(());
            }
        } else {
            let response = MCPResponse::Error {
                error: "Authentication required".to_string(),
                code: error_codes::UNAUTHORIZED,
            };
            send_response(&mut stream, &response).await?;
            return Ok(());
        }
    } else {
        None
    };
    
    // Handle request
    let response = handlers.handle_request(request, auth).await;
    
    // Send response
    send_response(&mut stream, &response).await?;
    
    Ok(())
}

/// Send response to client
async fn send_response(stream: &mut TcpStream, response: &MCPResponse) -> Result<()> {
    let data = serde_json::to_vec(response)?;
    stream.write_all(&data).await?;
    stream.flush().await?;
    Ok(())
}

/// Extract auth token from request (simplified)
fn extract_auth_token(data: &[u8]) -> Option<String> {
    // In production, parse proper HTTP headers
    // For now, look for "auth:" in the JSON
    let text = String::from_utf8_lossy(data);
    if let Some(pos) = text.find("\"auth\":\"") {
        let start = pos + 8;
        if let Some(end) = text[start..].find('"') {
            return Some(text[start..start+end].to_string());
        }
    }
    None
}