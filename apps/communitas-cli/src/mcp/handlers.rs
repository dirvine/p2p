// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// MCP request handlers

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use super::{MCPRequest, MCPResponse, MCPAuth, error_codes};
use crate::dht::DHTManager;
use crate::geographic::GeographicBootstrapManager;
use serde_json::json;

/// MCP request handlers
pub struct MCPHandlers {
    dht_manager: Arc<RwLock<Option<DHTManager>>>,
    geo_manager: Arc<RwLock<Option<GeographicBootstrapManager>>>,
}

impl MCPHandlers {
    /// Create new handlers
    pub fn new(
        dht_manager: Arc<RwLock<Option<DHTManager>>>,
        geo_manager: Arc<RwLock<Option<GeographicBootstrapManager>>>,
    ) -> Self {
        Self {
            dht_manager,
            geo_manager,
        }
    }
    
    /// Handle an MCP request
    pub async fn handle_request(&self, request: MCPRequest, auth: Option<MCPAuth>) -> MCPResponse {
        // Check permissions for protected endpoints
        if let Some(auth) = &auth {
            match &request {
                MCPRequest::DHTPut { .. } | 
                MCPRequest::ConfigUpdate { .. } |
                MCPRequest::Execute { .. } => {
                    if !auth.has_permission("write") {
                        return MCPResponse::Error {
                            error: "Insufficient permissions".to_string(),
                            code: error_codes::UNAUTHORIZED,
                        };
                    }
                }
                _ => {}
            }
        }
        
        match request {
            MCPRequest::NodeStatus => self.handle_node_status().await,
            MCPRequest::DHTStats => self.handle_dht_stats().await,
            MCPRequest::DHTPut { key, value, ttl } => {
                self.handle_dht_put(key, value, ttl).await
            }
            MCPRequest::DHTGet { key } => self.handle_dht_get(key).await,
            MCPRequest::DHTList { prefix, limit } => {
                self.handle_dht_list(prefix, limit).await
            }
            MCPRequest::GeoStatus => self.handle_geo_status().await,
            MCPRequest::GeoPeers { region } => self.handle_geo_peers(region).await,
            MCPRequest::ConfigUpdate { settings } => {
                self.handle_config_update(settings).await
            }
            MCPRequest::Execute { command, args } => {
                self.handle_execute(command, args).await
            }
            MCPRequest::Health => self.handle_health().await,
        }
    }
    
    /// Handle node status request
    async fn handle_node_status(&self) -> MCPResponse {
        let mut status = json!({
            "online": true,
            "version": env!("CARGO_PKG_VERSION"),
            "uptime": 0, // TODO: Track uptime
        });
        
        // Add DHT status if available
        if let Some(dht) = self.dht_manager.read().await.as_ref() {
            if let Ok(stats) = dht.get_stats().await {
                status["dht"] = json!({
                    "total_records": stats.total_records,
                    "storage_used_mb": stats.storage_used_mb,
                });
            }
        }
        
        // Add geographic status if available
        if let Some(geo) = self.geo_manager.read().await.as_ref() {
            if let Ok(status_info) = geo.get_status().await {
                status["geographic"] = json!({
                    "local_region": format!("{:?}", status_info.local_region),
                    "total_peers": status_info.total_peers,
                    "cross_region_connections": status_info.cross_region_connections,
                });
            }
        }
        
        MCPResponse::Success {
            success: true,
            data: status,
        }
    }
    
    /// Handle DHT stats request
    async fn handle_dht_stats(&self) -> MCPResponse {
        let dht_guard = self.dht_manager.read().await;
        
        if let Some(dht) = dht_guard.as_ref() {
            match dht.get_stats().await {
                Ok(stats) => MCPResponse::Success {
                    success: true,
                    data: serde_json::to_value(stats).unwrap_or(json!({})),
                },
                Err(e) => MCPResponse::Error {
                    error: format!("Failed to get DHT stats: {}", e),
                    code: error_codes::INTERNAL_ERROR,
                },
            }
        } else {
            MCPResponse::Error {
                error: "DHT not initialized".to_string(),
                code: error_codes::RESOURCE_NOT_FOUND,
            }
        }
    }
    
    /// Handle DHT put request
    async fn handle_dht_put(&self, key: String, value: String, ttl: Option<u64>) -> MCPResponse {
        let mut dht_guard = self.dht_manager.write().await;
        
        if let Some(dht) = dht_guard.as_mut() {
            let ttl = ttl.unwrap_or(86400);
            match dht.put(&key, value.into_bytes(), ttl).await {
                Ok(result) => MCPResponse::Success {
                    success: true,
                    data: json!({
                        "key": key,
                        "hash": result.hash,
                        "size": result.size,
                        "replicas": result.replicas,
                    }),
                },
                Err(e) => MCPResponse::Error {
                    error: format!("Failed to store value: {}", e),
                    code: error_codes::INTERNAL_ERROR,
                },
            }
        } else {
            MCPResponse::Error {
                error: "DHT not initialized".to_string(),
                code: error_codes::RESOURCE_NOT_FOUND,
            }
        }
    }
    
    /// Handle DHT get request
    async fn handle_dht_get(&self, key: String) -> MCPResponse {
        let mut dht_guard = self.dht_manager.write().await;
        
        if let Some(dht) = dht_guard.as_mut() {
            match dht.get(&key).await {
                Ok(Some(value)) => {
                    // Try to convert to string, otherwise base64
                    let value_str = match String::from_utf8(value.clone()) {
                        Ok(s) => s,
                        Err(_) => base64::encode(&value),
                    };
                    
                    MCPResponse::Success {
                        success: true,
                        data: json!({
                            "key": key,
                            "value": value_str,
                            "size": value.len(),
                        }),
                    }
                }
                Ok(None) => MCPResponse::Error {
                    error: format!("Key not found: {}", key),
                    code: error_codes::RESOURCE_NOT_FOUND,
                },
                Err(e) => MCPResponse::Error {
                    error: format!("Failed to retrieve value: {}", e),
                    code: error_codes::INTERNAL_ERROR,
                },
            }
        } else {
            MCPResponse::Error {
                error: "DHT not initialized".to_string(),
                code: error_codes::RESOURCE_NOT_FOUND,
            }
        }
    }
    
    /// Handle DHT list request
    async fn handle_dht_list(&self, prefix: Option<String>, limit: Option<usize>) -> MCPResponse {
        let dht_guard = self.dht_manager.read().await;
        
        if let Some(dht) = dht_guard.as_ref() {
            let limit = limit.unwrap_or(100);
            match dht.list_keys(prefix.as_deref(), limit).await {
                Ok(keys) => {
                    let key_list: Vec<_> = keys.into_iter()
                        .map(|(k, info)| json!({
                            "key": k,
                            "size": info.size,
                            "ttl": info.ttl,
                        }))
                        .collect();
                    
                    MCPResponse::Success {
                        success: true,
                        data: json!({
                            "keys": key_list,
                            "count": key_list.len(),
                        }),
                    }
                }
                Err(e) => MCPResponse::Error {
                    error: format!("Failed to list keys: {}", e),
                    code: error_codes::INTERNAL_ERROR,
                },
            }
        } else {
            MCPResponse::Error {
                error: "DHT not initialized".to_string(),
                code: error_codes::RESOURCE_NOT_FOUND,
            }
        }
    }
    
    /// Handle geographic status request
    async fn handle_geo_status(&self) -> MCPResponse {
        let geo_guard = self.geo_manager.read().await;
        
        if let Some(geo) = geo_guard.as_ref() {
            match geo.get_status().await {
                Ok(status) => MCPResponse::Success {
                    success: true,
                    data: json!({
                        "local_region": format!("{:?}", status.local_region),
                        "active_regions": status.active_regions,
                        "total_peers": status.total_peers,
                        "cross_region_connections": status.cross_region_connections,
                        "avg_latency_ms": status.avg_latency_ms,
                        "regional_distribution": status.regional_distribution.iter()
                            .map(|(r, c)| json!({
                                "region": format!("{:?}", r),
                                "count": c,
                            }))
                            .collect::<Vec<_>>(),
                    }),
                },
                Err(e) => MCPResponse::Error {
                    error: format!("Failed to get geographic status: {}", e),
                    code: error_codes::INTERNAL_ERROR,
                },
            }
        } else {
            MCPResponse::Error {
                error: "Geographic routing not initialized".to_string(),
                code: error_codes::RESOURCE_NOT_FOUND,
            }
        }
    }
    
    /// Handle geographic peers request
    async fn handle_geo_peers(&self, region: Option<String>) -> MCPResponse {
        let geo_guard = self.geo_manager.read().await;
        
        if let Some(geo) = geo_guard.as_ref() {
            let region_filter = if let Some(r) = region {
                match crate::geographic::commands::parse_region(&r) {
                    Ok(region) => Some(region),
                    Err(e) => {
                        return MCPResponse::Error {
                            error: format!("Invalid region: {}", e),
                            code: error_codes::INVALID_PARAMS,
                        };
                    }
                }
            } else {
                None
            };
            
            match geo.get_peers_by_region(region_filter).await {
                Ok(peers) => {
                    let peer_list: Vec<_> = peers.into_iter()
                        .map(|p| json!({
                            "id": p.id,
                            "region": format!("{:?}", p.region),
                            "location": p.location,
                            "latency_ms": p.latency_ms,
                            "reliability": p.reliability,
                        }))
                        .collect();
                    
                    MCPResponse::Success {
                        success: true,
                        data: json!({
                            "peers": peer_list,
                            "count": peer_list.len(),
                        }),
                    }
                }
                Err(e) => MCPResponse::Error {
                    error: format!("Failed to get peers: {}", e),
                    code: error_codes::INTERNAL_ERROR,
                },
            }
        } else {
            MCPResponse::Error {
                error: "Geographic routing not initialized".to_string(),
                code: error_codes::RESOURCE_NOT_FOUND,
            }
        }
    }
    
    /// Handle config update request
    async fn handle_config_update(&self, settings: std::collections::HashMap<String, serde_json::Value>) -> MCPResponse {
        // TODO: Implement configuration updates
        MCPResponse::Success {
            success: true,
            data: json!({
                "updated": settings.keys().cloned().collect::<Vec<_>>(),
            }),
        }
    }
    
    /// Handle execute command request
    async fn handle_execute(&self, command: String, args: Vec<String>) -> MCPResponse {
        // Security: Only allow specific whitelisted commands
        let allowed_commands = vec!["status", "stats", "ping"];
        
        if !allowed_commands.contains(&command.as_str()) {
            return MCPResponse::Error {
                error: format!("Command not allowed: {}", command),
                code: error_codes::INVALID_REQUEST,
            };
        }
        
        // TODO: Execute command
        MCPResponse::Success {
            success: true,
            data: json!({
                "command": command,
                "args": args,
                "output": "Command execution not yet implemented",
            }),
        }
    }
    
    /// Handle health check
    async fn handle_health(&self) -> MCPResponse {
        MCPResponse::Success {
            success: true,
            data: json!({
                "status": "healthy",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        }
    }
}

// Helper function to parse region strings
use saorsa_core::network::geographic::GeographicRegion;

fn parse_region(s: &str) -> Result<GeographicRegion> {
    match s.to_lowercase().as_str() {
        "na" | "northamerica" => Ok(GeographicRegion::NorthAmerica),
        "eu" | "europe" => Ok(GeographicRegion::Europe),
        "ap" | "asiapacific" => Ok(GeographicRegion::AsiaPacific),
        "sa" | "southamerica" => Ok(GeographicRegion::SouthAmerica),
        "af" | "africa" => Ok(GeographicRegion::Africa),
        "oc" | "oceania" => Ok(GeographicRegion::Oceania),
        _ => Ok(GeographicRegion::Unknown),
    }
}