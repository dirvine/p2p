// Copyright 2025 Saorsa Labs Limited  
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// MCP (Model Context Protocol) server for remote management

pub mod server;
pub mod handlers;

pub use server::{MCPServer, MCPConfig};
pub use handlers::MCPHandlers;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MCP request types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum MCPRequest {
    /// Get node status
    #[serde(rename = "node/status")]
    NodeStatus,
    
    /// Get DHT statistics
    #[serde(rename = "dht/stats")]
    DHTStats,
    
    /// Store value in DHT
    #[serde(rename = "dht/put")]
    DHTPut {
        key: String,
        value: String,
        ttl: Option<u64>,
    },
    
    /// Retrieve value from DHT
    #[serde(rename = "dht/get")]
    DHTGet {
        key: String,
    },
    
    /// List DHT keys
    #[serde(rename = "dht/list")]
    DHTList {
        prefix: Option<String>,
        limit: Option<usize>,
    },
    
    /// Get geographic status
    #[serde(rename = "geo/status")]
    GeoStatus,
    
    /// Get peers by region
    #[serde(rename = "geo/peers")]
    GeoPeers {
        region: Option<String>,
    },
    
    /// Configure node
    #[serde(rename = "config/update")]
    ConfigUpdate {
        settings: HashMap<String, serde_json::Value>,
    },
    
    /// Execute command
    #[serde(rename = "exec")]
    Execute {
        command: String,
        args: Vec<String>,
    },
    
    /// Health check
    #[serde(rename = "health")]
    Health,
}

/// MCP response types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MCPResponse {
    Success {
        success: bool,
        data: serde_json::Value,
    },
    Error {
        error: String,
        code: i32,
    },
}

/// MCP authentication token
#[derive(Debug, Clone)]
pub struct MCPAuth {
    pub token: String,
    pub permissions: Vec<String>,
}

impl MCPAuth {
    /// Create a new auth token
    pub fn new(token: String, permissions: Vec<String>) -> Self {
        Self { token, permissions }
    }
    
    /// Check if has permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string()) 
            || self.permissions.contains(&"*".to_string())
    }
}

/// MCP error codes
pub mod error_codes {
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    pub const UNAUTHORIZED: i32 = -32604;
    pub const RESOURCE_NOT_FOUND: i32 = -32605;
}