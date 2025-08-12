// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Geographic routing for CLI bootstrap nodes

pub mod commands;
pub mod manager;

pub use commands::{GeographicCommands, execute_geographic_command};
pub use manager::{GeographicBootstrapManager, RegionStats};

// Define local geographic types since they don't exist in saorsa-core yet
use std::net::IpAddr;

/// Geographic regions for bootstrap node distribution
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum GeographicRegion {
    NorthAmerica,
    Europe,
    AsiaPacific,
    SouthAmerica,
    Africa,
    Oceania,
    Unknown,
}

/// Geographic location information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GeographicLocation {
    pub region: GeographicRegion,
    pub country: String,
    pub city: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

/// Geographic network service for handling region-specific operations
#[derive(Debug)]
pub struct GeographicNetworkService {
    initialized: bool,
}

impl GeographicNetworkService {
    pub fn new() -> Self {
        Self {
            initialized: false,
        }
    }
    
    pub async fn initialize(&mut self) -> anyhow::Result<()> {
        self.initialized = true;
        Ok(())
    }
}

/// Geographic bootstrap configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GeographicBootstrapConfig {
    /// Local region (auto-detected or configured)
    pub local_region: GeographicRegion,
    /// Enable cross-region optimization
    pub cross_region_optimization: bool,
    /// Preferred regions for routing
    pub preferred_regions: Vec<GeographicRegion>,
    /// Maximum cross-region connections
    pub max_cross_region: usize,
    /// Latency threshold for region preference (ms)
    pub latency_threshold_ms: u64,
    /// Enable region-aware replication
    pub regional_replication: bool,
}

impl Default for GeographicBootstrapConfig {
    fn default() -> Self {
        Self {
            local_region: GeographicRegion::Unknown,
            cross_region_optimization: true,
            preferred_regions: vec![],
            max_cross_region: 10,
            latency_threshold_ms: 100,
            regional_replication: true,
        }
    }
}

/// Detect geographic region from IP address
pub fn detect_region(ip: &IpAddr) -> GeographicRegion {
    // Simple IP-based region detection
    // In production, use GeoIP database
    match ip {
        IpAddr::V4(ipv4) => {
            let first_octet = ipv4.octets()[0];
            match first_octet {
                1..=50 => GeographicRegion::NorthAmerica,
                51..=100 => GeographicRegion::Europe,
                101..=150 => GeographicRegion::AsiaPacific,
                151..=180 => GeographicRegion::SouthAmerica,
                181..=200 => GeographicRegion::Africa,
                201..=220 => GeographicRegion::Oceania,
                _ => {
                    // Special case for known IPs
                    if ipv4.to_string().starts_with("159.89.") {
                        GeographicRegion::Europe // DigitalOcean Europe
                    } else {
                        GeographicRegion::Unknown
                    }
                }
            }
        }
        IpAddr::V6(_) => GeographicRegion::Unknown,
    }
}