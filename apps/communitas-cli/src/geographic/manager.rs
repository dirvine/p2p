// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Geographic routing manager for CLI bootstrap nodes

use anyhow::Result;
use super::{GeographicRegion, GeographicLocation, GeographicNetworkService};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::SystemTime;
use super::GeographicBootstrapConfig;

/// Geographic routing status
#[derive(Debug, Clone, serde::Serialize)]
pub struct GeographicStatus {
    pub local_region: GeographicRegion,
    pub active_regions: usize,
    pub total_peers: usize,
    pub cross_region_connections: usize,
    pub avg_latency_ms: f64,
    pub regional_distribution: HashMap<GeographicRegion, usize>,
    pub cross_region_links: HashMap<String, usize>,
}

/// Geographic peer information
#[derive(Debug, Clone, serde::Serialize)]
pub struct GeographicPeer {
    pub id: String,
    pub region: GeographicRegion,
    pub location: String,
    pub latency_ms: u64,
    pub reliability: f64,
    pub last_seen: SystemTime,
}

/// Regional statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct RegionStats {
    pub region: GeographicRegion,
    pub peer_count: usize,
    pub avg_latency_ms: f64,
    pub success_rate: f64,
    pub avg_bandwidth_mbps: f64,
}

/// Connectivity test results
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConnectivityTestResult {
    pub tested_count: usize,
    pub successful: usize,
    pub failed: usize,
    pub avg_latency_ms: f64,
    pub min_latency_ms: u64,
    pub max_latency_ms: u64,
    pub packet_loss: f64,
}

/// Optimization result
#[derive(Debug, Clone, serde::Serialize)]
pub struct OptimizationResult {
    pub connections_added: usize,
    pub connections_removed: usize,
    pub connections_optimized: usize,
    pub latency_improvement: f64,
    pub changes: Vec<String>,
}

/// Geographic bootstrap manager
pub struct GeographicBootstrapManager {
    config: Arc<RwLock<GeographicBootstrapConfig>>,
    network_service: Arc<RwLock<GeographicNetworkService>>,
    peer_cache: Arc<RwLock<HashMap<String, GeographicPeer>>>,
    latency_map: Arc<RwLock<HashMap<String, f64>>>,
}

impl GeographicBootstrapManager {
    /// Create a new geographic bootstrap manager
    pub async fn new(config: GeographicBootstrapConfig) -> Result<Self> {
        let network_service = GeographicNetworkService::new();
        
        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            network_service: Arc::new(RwLock::new(network_service)),
            peer_cache: Arc::new(RwLock::new(HashMap::new())),
            latency_map: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Initialize the manager
    pub async fn initialize(&mut self) -> Result<()> {
        // Detect local region if not configured
        let mut config = self.config.write().await;
        if matches!(config.local_region, GeographicRegion::Unknown) {
            // Try to detect from public IP
            if let Ok(public_ip) = self.get_public_ip().await {
                config.local_region = super::detect_region(&public_ip);
            }
        }
        
        // Initialize network service
        let mut service = self.network_service.write().await;
        service.initialize().await?;
        
        // Load initial latency map
        self.initialize_latency_map().await;
        
        Ok(())
    }
    
    /// Get public IP address
    async fn get_public_ip(&self) -> Result<std::net::IpAddr> {
        // In production, use an IP detection service
        // For now, return a placeholder
        Ok("159.89.81.21".parse()?)
    }
    
    /// Initialize latency map with typical values
    async fn initialize_latency_map(&self) {
        let mut map = self.latency_map.write().await;
        
        // Typical inter-region latencies (ms)
        let latencies = [
            ("NorthAmerica_Europe", 80.0),
            ("NorthAmerica_AsiaPacific", 150.0),
            ("NorthAmerica_SouthAmerica", 120.0),
            ("Europe_AsiaPacific", 200.0),
            ("Europe_Africa", 60.0),
            ("AsiaPacific_Oceania", 50.0),
            // Add more as needed
        ];
        
        for (route, latency) in latencies {
            map.insert(route.to_string(), latency);
            // Add reverse route
            let parts: Vec<&str> = route.split('_').collect();
            if parts.len() == 2 {
                let reverse = format!("{}_{}", parts[1], parts[0]);
                map.insert(reverse, latency);
            }
        }
    }
    
    /// Get geographic routing status
    pub async fn get_status(&self) -> Result<GeographicStatus> {
        let config = self.config.read().await;
        let peers = self.peer_cache.read().await;
        
        let mut regional_distribution = HashMap::new();
        let mut cross_region_links = HashMap::new();
        let mut total_latency = 0.0;
        let mut cross_region_count = 0;
        
        for peer in peers.values() {
            *regional_distribution.entry(peer.region.clone()).or_insert(0) += 1;
            total_latency += peer.latency_ms as f64;
            
            if peer.region != config.local_region {
                cross_region_count += 1;
                let link = format!("{:?}_{:?}", config.local_region, peer.region);
                *cross_region_links.entry(link).or_insert(0) += 1;
            }
        }
        
        Ok(GeographicStatus {
            local_region: config.local_region.clone(),
            active_regions: regional_distribution.len(),
            total_peers: peers.len(),
            cross_region_connections: cross_region_count,
            avg_latency_ms: if peers.is_empty() { 0.0 } else { total_latency / peers.len() as f64 },
            regional_distribution,
            cross_region_links,
        })
    }
    
    /// Get peers by region
    pub async fn get_peers_by_region(&self, region: Option<GeographicRegion>) -> Result<Vec<GeographicPeer>> {
        let peers = self.peer_cache.read().await;
        
        let result: Vec<GeographicPeer> = if let Some(r) = region {
            peers.values()
                .filter(|p| p.region == r)
                .cloned()
                .collect()
        } else {
            peers.values().cloned().collect()
        };
        
        Ok(result)
    }
    
    /// Get regional statistics
    pub async fn get_regional_stats(&self) -> Result<Vec<RegionStats>> {
        let peers = self.peer_cache.read().await;
        let mut stats_map: HashMap<GeographicRegion, (usize, f64, f64, f64)> = HashMap::new();
        
        for peer in peers.values() {
            let entry = stats_map.entry(peer.region.clone()).or_insert((0, 0.0, 0.0, 0.0));
            entry.0 += 1; // count
            entry.1 += peer.latency_ms as f64; // total latency
            entry.2 += peer.reliability; // total reliability
            entry.3 += 100.0; // placeholder bandwidth
        }
        
        let mut stats = Vec::new();
        for (region, (count, total_latency, total_reliability, total_bandwidth)) in stats_map {
            stats.push(RegionStats {
                region,
                peer_count: count,
                avg_latency_ms: if count > 0 { total_latency / count as f64 } else { 0.0 },
                success_rate: if count > 0 { total_reliability / count as f64 } else { 0.0 },
                avg_bandwidth_mbps: if count > 0 { total_bandwidth / count as f64 } else { 0.0 },
            });
        }
        
        Ok(stats)
    }
    
    /// Get configuration
    pub async fn get_config(&self) -> Result<GeographicBootstrapConfig> {
        Ok(self.config.read().await.clone())
    }
    
    /// Set local region
    pub async fn set_local_region(&mut self, region: GeographicRegion) -> Result<()> {
        let mut config = self.config.write().await;
        config.local_region = region;
        Ok(())
    }
    
    /// Set cross-region optimization
    pub async fn set_cross_region_optimization(&mut self, enabled: bool) -> Result<()> {
        let mut config = self.config.write().await;
        config.cross_region_optimization = enabled;
        Ok(())
    }
    
    /// Set latency threshold
    pub async fn set_latency_threshold(&mut self, threshold_ms: u64) -> Result<()> {
        let mut config = self.config.write().await;
        config.latency_threshold_ms = threshold_ms;
        Ok(())
    }
    
    /// Add preferred region
    pub async fn add_preferred_region(&mut self, region: GeographicRegion) -> Result<()> {
        let mut config = self.config.write().await;
        if !config.preferred_regions.contains(&region) {
            config.preferred_regions.push(region);
        }
        Ok(())
    }
    
    /// Remove preferred region
    pub async fn remove_preferred_region(&mut self, region: GeographicRegion) -> Result<()> {
        let mut config = self.config.write().await;
        config.preferred_regions.retain(|r| r != &region);
        Ok(())
    }
    
    /// Test connectivity to a region
    pub async fn test_region_connectivity(
        &mut self,
        region: GeographicRegion,
        count: usize,
    ) -> Result<ConnectivityTestResult> {
        let peers = self.get_peers_by_region(Some(region)).await?;
        let test_peers: Vec<_> = peers.iter().take(count).collect();
        
        let mut successful = 0;
        let mut failed = 0;
        let mut total_latency = 0.0;
        let mut min_latency = u64::MAX;
        let mut max_latency = 0u64;
        
        for peer in test_peers.iter() {
            // Simulate connectivity test
            // In production, actually ping the peer
            if peer.reliability > 0.5 {
                successful += 1;
                total_latency += peer.latency_ms as f64;
                min_latency = min_latency.min(peer.latency_ms);
                max_latency = max_latency.max(peer.latency_ms);
            } else {
                failed += 1;
            }
        }
        
        let tested_count = test_peers.len();
        
        Ok(ConnectivityTestResult {
            tested_count,
            successful,
            failed,
            avg_latency_ms: if successful > 0 { total_latency / successful as f64 } else { 0.0 },
            min_latency_ms: if min_latency == u64::MAX { 0 } else { min_latency },
            max_latency_ms: max_latency,
            packet_loss: if tested_count > 0 { failed as f64 / tested_count as f64 } else { 0.0 },
        })
    }
    
    /// Optimize routing for geographic distribution
    pub async fn optimize_routing(&mut self, dry_run: bool) -> Result<OptimizationResult> {
        let config = self.config.read().await;
        let mut peers = self.peer_cache.write().await;
        
        let mut result = OptimizationResult {
            connections_added: 0,
            connections_removed: 0,
            connections_optimized: 0,
            latency_improvement: 0.0,
            changes: Vec::new(),
        };
        
        // Identify peers to optimize
        let mut to_remove = Vec::new();
        let mut to_optimize = Vec::new();
        
        for (id, peer) in peers.iter() {
            // Remove high-latency cross-region connections
            if peer.region != config.local_region 
                && peer.latency_ms > config.latency_threshold_ms 
                && !config.preferred_regions.contains(&peer.region) {
                to_remove.push(id.clone());
                result.changes.push(format!(
                    "Remove high-latency connection to {} ({:?}, {}ms)",
                    id, peer.region, peer.latency_ms
                ));
            }
            
            // Optimize connections with poor reliability
            if peer.reliability < 0.7 {
                to_optimize.push(id.clone());
                result.changes.push(format!(
                    "Optimize connection to {} (reliability: {:.1}%)",
                    id, peer.reliability * 100.0
                ));
            }
        }
        
        if !dry_run {
            // Actually perform optimizations
            for id in to_remove {
                peers.remove(&id);
                result.connections_removed += 1;
            }
            
            result.connections_optimized = to_optimize.len();
            
            // TODO: Add new connections to preferred regions
            // This would involve discovering new peers in preferred regions
        }
        
        // Calculate estimated improvement
        if result.connections_removed > 0 {
            result.latency_improvement = 20.0; // Placeholder
        }
        
        Ok(result)
    }
    
    /// Get latency map between regions
    pub async fn get_latency_map(&self) -> Result<HashMap<String, f64>> {
        Ok(self.latency_map.read().await.clone())
    }
    
    /// Add a peer to the cache
    pub async fn add_peer(&mut self, peer: GeographicPeer) -> Result<()> {
        let mut peers = self.peer_cache.write().await;
        peers.insert(peer.id.clone(), peer);
        Ok(())
    }
    
    /// Update peer metrics
    pub async fn update_peer_metrics(&mut self, id: &str, latency_ms: u64, success: bool) -> Result<()> {
        let mut peers = self.peer_cache.write().await;
        
        if let Some(peer) = peers.get_mut(id) {
            peer.latency_ms = latency_ms;
            peer.last_seen = SystemTime::now();
            
            // Update reliability (simple exponential moving average)
            let success_value = if success { 1.0 } else { 0.0 };
            peer.reliability = peer.reliability * 0.9 + success_value * 0.1;
        }
        
        Ok(())
    }
}