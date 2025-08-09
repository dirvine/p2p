// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Connection management for P2P networking

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Connection information
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub peer_address: String,
    pub connected_at: Instant,
    pub last_activity: Instant,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub latency_ms: Option<u32>,
}

/// Connection state
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Connecting,
    Connected,
    Disconnecting,
    Disconnected,
    Failed(String),
}

/// Manages P2P connections
#[derive(Debug)]
pub struct ConnectionManager {
    connections: Arc<RwLock<HashMap<String, ConnectionInfo>>>,
    max_connections: usize,
    connection_timeout: Duration,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new(max_connections: usize, timeout_secs: u64) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            max_connections,
            connection_timeout: Duration::from_secs(timeout_secs),
        }
    }

    /// Add a new connection
    pub async fn add_connection(&self, peer_address: String) -> Result<()> {
        let mut connections = self.connections.write().await;
        
        if connections.len() >= self.max_connections {
            anyhow::bail!("Maximum connections ({}) reached", self.max_connections);
        }

        if connections.contains_key(&peer_address) {
            anyhow::bail!("Connection to {} already exists", peer_address);
        }

        let info = ConnectionInfo {
            peer_address: peer_address.clone(),
            connected_at: Instant::now(),
            last_activity: Instant::now(),
            bytes_sent: 0,
            bytes_received: 0,
            latency_ms: None,
        };

        connections.insert(peer_address, info);
        Ok(())
    }

    /// Remove a connection
    pub async fn remove_connection(&self, peer_address: &str) -> Result<()> {
        let mut connections = self.connections.write().await;
        connections.remove(peer_address)
            .ok_or_else(|| anyhow::anyhow!("Connection to {} not found", peer_address))?;
        Ok(())
    }

    /// Get connection info
    pub async fn get_connection(&self, peer_address: &str) -> Option<ConnectionInfo> {
        let connections = self.connections.read().await;
        connections.get(peer_address).cloned()
    }

    /// Get all connections
    pub async fn get_all_connections(&self) -> Vec<ConnectionInfo> {
        let connections = self.connections.read().await;
        connections.values().cloned().collect()
    }

    /// Get connection count
    pub async fn get_connection_count(&self) -> usize {
        self.connections.read().await.len()
    }

    /// Update connection activity
    pub async fn update_activity(&self, peer_address: &str) -> Result<()> {
        let mut connections = self.connections.write().await;
        
        if let Some(info) = connections.get_mut(peer_address) {
            info.last_activity = Instant::now();
            Ok(())
        } else {
            anyhow::bail!("Connection to {} not found", peer_address)
        }
    }

    /// Update connection statistics
    pub async fn update_stats(
        &self,
        peer_address: &str,
        bytes_sent: u64,
        bytes_received: u64,
        latency_ms: Option<u32>,
    ) -> Result<()> {
        let mut connections = self.connections.write().await;
        
        if let Some(info) = connections.get_mut(peer_address) {
            info.bytes_sent += bytes_sent;
            info.bytes_received += bytes_received;
            if let Some(latency) = latency_ms {
                info.latency_ms = Some(latency);
            }
            info.last_activity = Instant::now();
            Ok(())
        } else {
            anyhow::bail!("Connection to {} not found", peer_address)
        }
    }

    /// Clean up stale connections
    pub async fn cleanup_stale_connections(&self) -> Vec<String> {
        let mut connections = self.connections.write().await;
        let now = Instant::now();
        let mut removed = Vec::new();

        connections.retain(|addr, info| {
            let is_stale = now.duration_since(info.last_activity) > self.connection_timeout;
            if is_stale {
                removed.push(addr.clone());
            }
            !is_stale
        });

        removed
    }

    /// Check if can accept more connections
    pub async fn can_accept_connection(&self) -> bool {
        self.connections.read().await.len() < self.max_connections
    }

    /// Get connection health status
    pub async fn get_connection_health(&self, peer_address: &str) -> Result<ConnectionHealth> {
        let connections = self.connections.read().await;
        
        if let Some(info) = connections.get(peer_address) {
            let now = Instant::now();
            let idle_time = now.duration_since(info.last_activity);
            
            let health = if idle_time < Duration::from_secs(30) {
                ConnectionHealth::Healthy
            } else if idle_time < Duration::from_secs(120) {
                ConnectionHealth::Warning
            } else {
                ConnectionHealth::Unhealthy
            };
            
            Ok(health)
        } else {
            anyhow::bail!("Connection to {} not found", peer_address)
        }
    }
}

/// Connection health status
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionHealth {
    Healthy,
    Warning,
    Unhealthy,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_manager_creation() {
        let manager = ConnectionManager::new(10, 30);
        assert_eq!(manager.get_connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_add_remove_connection() {
        let manager = ConnectionManager::new(10, 30);
        
        // Add connection
        manager.add_connection("127.0.0.1:9000".to_string()).await.unwrap();
        assert_eq!(manager.get_connection_count().await, 1);
        
        // Get connection
        let info = manager.get_connection("127.0.0.1:9000").await;
        assert!(info.is_some());
        
        // Remove connection
        manager.remove_connection("127.0.0.1:9000").await.unwrap();
        assert_eq!(manager.get_connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_max_connections() {
        let manager = ConnectionManager::new(2, 30);
        
        // Add connections up to max
        manager.add_connection("127.0.0.1:9001".to_string()).await.unwrap();
        manager.add_connection("127.0.0.1:9002".to_string()).await.unwrap();
        
        // Try to add one more
        let result = manager.add_connection("127.0.0.1:9003".to_string()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Maximum connections"));
    }

    #[tokio::test]
    async fn test_duplicate_connection() {
        let manager = ConnectionManager::new(10, 30);
        
        manager.add_connection("127.0.0.1:9000".to_string()).await.unwrap();
        
        // Try to add duplicate
        let result = manager.add_connection("127.0.0.1:9000".to_string()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_update_activity() {
        let manager = ConnectionManager::new(10, 30);
        
        manager.add_connection("127.0.0.1:9000".to_string()).await.unwrap();
        
        // Update activity
        let result = manager.update_activity("127.0.0.1:9000").await;
        assert!(result.is_ok());
        
        // Update non-existent connection
        let result = manager.update_activity("127.0.0.1:9999").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_stats() {
        let manager = ConnectionManager::new(10, 30);
        
        manager.add_connection("127.0.0.1:9000".to_string()).await.unwrap();
        
        // Update stats
        manager.update_stats("127.0.0.1:9000", 100, 200, Some(50)).await.unwrap();
        
        let info = manager.get_connection("127.0.0.1:9000").await.unwrap();
        assert_eq!(info.bytes_sent, 100);
        assert_eq!(info.bytes_received, 200);
        assert_eq!(info.latency_ms, Some(50));
    }

    #[tokio::test]
    async fn test_connection_health() {
        let manager = ConnectionManager::new(10, 30);
        
        manager.add_connection("127.0.0.1:9000".to_string()).await.unwrap();
        
        // Fresh connection should be healthy
        let health = manager.get_connection_health("127.0.0.1:9000").await.unwrap();
        assert_eq!(health, ConnectionHealth::Healthy);
    }
}