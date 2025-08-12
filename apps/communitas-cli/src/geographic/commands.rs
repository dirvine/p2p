// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Geographic routing CLI commands

use clap::Subcommand;
use anyhow::Result;
use super::manager::GeographicBootstrapManager;
use super::{GeographicRegion};

#[derive(Subcommand, Debug)]
pub enum GeographicCommands {
    /// Show geographic routing status
    Status {
        /// Show detailed information
        #[arg(long)]
        detailed: bool,
    },
    
    /// List peers by region
    Peers {
        /// Filter by specific region
        #[arg(long)]
        region: Option<String>,
        /// Show detailed peer information
        #[arg(long)]
        detailed: bool,
    },
    
    /// Show regional statistics
    Stats {
        /// Output format (json, table)
        #[arg(long, default_value = "table")]
        format: String,
    },
    
    /// Configure geographic routing
    Config {
        /// Set local region
        #[arg(long)]
        region: Option<String>,
        /// Enable/disable cross-region optimization
        #[arg(long)]
        cross_region: Option<bool>,
        /// Set latency threshold (ms)
        #[arg(long)]
        latency_threshold: Option<u64>,
        /// Add preferred region
        #[arg(long)]
        add_preferred: Option<String>,
        /// Remove preferred region
        #[arg(long)]
        remove_preferred: Option<String>,
        /// Show current configuration
        #[arg(long)]
        show: bool,
    },
    
    /// Test connectivity to a region
    Test {
        /// Target region to test
        region: String,
        /// Number of peers to test
        #[arg(long, default_value = "5")]
        count: usize,
    },
    
    /// Optimize routing tables for geographic distribution
    Optimize {
        /// Dry run without making changes
        #[arg(long)]
        dry_run: bool,
    },
    
    /// Show latency map between regions
    Latency {
        /// Show detailed latency matrix
        #[arg(long)]
        matrix: bool,
    },
}

pub async fn execute_geographic_command(
    manager: &mut GeographicBootstrapManager,
    command: GeographicCommands,
) -> Result<()> {
    use GeographicCommands::*;
    
    match command {
        Status { detailed } => {
            let status = manager.get_status().await?;
            
            println!("Geographic Routing Status");
            println!("{:-<50}", "");
            println!("Local Region: {:?}", status.local_region);
            println!("Active Regions: {}", status.active_regions);
            println!("Total Peers: {}", status.total_peers);
            println!("Cross-Region Connections: {}", status.cross_region_connections);
            println!("Average Latency: {:.2} ms", status.avg_latency_ms);
            
            if detailed {
                println!("\nRegional Distribution:");
                for (region, count) in status.regional_distribution {
                    println!("  {:?}: {} peers", region, count);
                }
                
                println!("\nCross-Region Links:");
                for (link, count) in status.cross_region_links {
                    println!("  {}: {} connections", link, count);
                }
            }
        }
        
        Peers { region, detailed } => {
            let region_filter = if let Some(r) = region {
                Some(parse_region(&r)?)
            } else {
                None
            };
            
            let peers = manager.get_peers_by_region(region_filter).await?;
            
            if detailed {
                println!("Geographic Peer Distribution");
                println!("{:-<80}", "");
                for peer in peers {
                    println!("Peer: {}", peer.id);
                    println!("  Region: {:?}", peer.region);
                    println!("  Location: {}", peer.location);
                    println!("  Latency: {} ms", peer.latency_ms);
                    println!("  Reliability: {:.2}%", peer.reliability * 100.0);
                    println!("  Last Seen: {:?}", peer.last_seen);
                    println!();
                }
            } else {
                println!("ID                          Region          Latency  Location");
                println!("{:-<80}", "");
                for peer in peers {
                    println!("{:<28} {:<15?} {:>6} ms  {}", 
                        &peer.id[..28.min(peer.id.len())],
                        peer.region,
                        peer.latency_ms,
                        peer.location);
                }
            }
        }
        
        Stats { format } => {
            let stats = manager.get_regional_stats().await?;
            
            if format == "json" {
                println!("{}", serde_json::to_string_pretty(&stats)?);
            } else {
                println!("Regional Statistics");
                println!("{:-<60}", "");
                println!("Region            Peers  Avg Latency  Success Rate  Bandwidth");
                println!("{:-<60}", "");
                
                for stat in stats {
                    println!("{:<17?} {:>5}  {:>8.1} ms  {:>10.1}%  {:>9.1} Mbps",
                        stat.region,
                        stat.peer_count,
                        stat.avg_latency_ms,
                        stat.success_rate * 100.0,
                        stat.avg_bandwidth_mbps);
                }
            }
        }
        
        Config { 
            region, 
            cross_region, 
            latency_threshold,
            add_preferred,
            remove_preferred,
            show 
        } => {
            if show {
                let config = manager.get_config().await?;
                println!("Geographic Routing Configuration");
                println!("{:-<50}", "");
                println!("Local Region: {:?}", config.local_region);
                println!("Cross-Region Optimization: {}", config.cross_region_optimization);
                println!("Latency Threshold: {} ms", config.latency_threshold_ms);
                println!("Max Cross-Region: {}", config.max_cross_region);
                println!("Regional Replication: {}", config.regional_replication);
                
                if !config.preferred_regions.is_empty() {
                    println!("Preferred Regions:");
                    for region in config.preferred_regions {
                        println!("  - {:?}", region);
                    }
                }
            } else {
                let mut updated = false;
                
                if let Some(r) = region {
                    let region = parse_region(&r)?;
                    manager.set_local_region(region.clone()).await?;
                    println!("✓ Set local region to {:?}", region);
                    updated = true;
                }
                
                if let Some(cr) = cross_region {
                    manager.set_cross_region_optimization(cr).await?;
                    println!("✓ Cross-region optimization {}", 
                        if cr { "enabled" } else { "disabled" });
                    updated = true;
                }
                
                if let Some(lt) = latency_threshold {
                    manager.set_latency_threshold(lt).await?;
                    println!("✓ Set latency threshold to {} ms", lt);
                    updated = true;
                }
                
                if let Some(ap) = add_preferred {
                    let region = parse_region(&ap)?;
                    manager.add_preferred_region(region.clone()).await?;
                    println!("✓ Added {:?} to preferred regions", region);
                    updated = true;
                }
                
                if let Some(rp) = remove_preferred {
                    let region = parse_region(&rp)?;
                    manager.remove_preferred_region(region.clone()).await?;
                    println!("✓ Removed {:?} from preferred regions", region);
                    updated = true;
                }
                
                if !updated {
                    println!("No configuration changes specified");
                }
            }
        }
        
        Test { region, count } => {
            let target_region = parse_region(&region)?;
            println!("Testing connectivity to {:?} region...", target_region);
            
            let results = manager.test_region_connectivity(target_region, count).await?;
            
            println!("\nConnectivity Test Results:");
            println!("{:-<60}", "");
            println!("Tested Peers: {}", results.tested_count);
            println!("Successful: {}", results.successful);
            println!("Failed: {}", results.failed);
            println!("Average Latency: {:.2} ms", results.avg_latency_ms);
            println!("Min Latency: {} ms", results.min_latency_ms);
            println!("Max Latency: {} ms", results.max_latency_ms);
            println!("Packet Loss: {:.2}%", results.packet_loss * 100.0);
        }
        
        Optimize { dry_run } => {
            println!("Optimizing geographic routing tables...");
            
            let result = manager.optimize_routing(dry_run).await?;
            
            if dry_run {
                println!("\nOptimization Preview (dry run):");
            } else {
                println!("\nOptimization Complete:");
            }
            
            println!("{:-<50}", "");
            println!("Connections Added: {}", result.connections_added);
            println!("Connections Removed: {}", result.connections_removed);
            println!("Connections Optimized: {}", result.connections_optimized);
            println!("Estimated Latency Improvement: {:.2} ms", result.latency_improvement);
            
            if !result.changes.is_empty() {
                println!("\nChanges:");
                for change in result.changes.iter().take(10) {
                    println!("  {}", change);
                }
                if result.changes.len() > 10 {
                    println!("  ... and {} more", result.changes.len() - 10);
                }
            }
        }
        
        Latency { matrix } => {
            let latencies = manager.get_latency_map().await?;
            
            if matrix {
                println!("Regional Latency Matrix (ms)");
                println!("{:-<100}", "");
                
                // Print header
                print!("{:15}", "From/To");
                for region in &["NA", "EU", "AP", "SA", "AF", "OC"] {
                    print!("{:>12}", region);
                }
                println!();
                println!("{:-<100}", "");
                
                // Print matrix
                let regions = vec![
                    ("NorthAmerica", "NA"),
                    ("Europe", "EU"),
                    ("AsiaPacific", "AP"),
                    ("SouthAmerica", "SA"),
                    ("Africa", "AF"),
                    ("Oceania", "OC"),
                ];
                
                for (from_name, from_abbr) in &regions {
                    print!("{:15}", from_abbr);
                    for (to_name, _) in &regions {
                        let key = format!("{}_{}", from_name, to_name);
                        if let Some(latency) = latencies.get(&key) {
                            print!("{:>12.1}", latency);
                        } else {
                            print!("{:>12}", "-");
                        }
                    }
                    println!();
                }
            } else {
                println!("Regional Latency Summary");
                println!("{:-<60}", "");
                println!("Route                                    Latency (ms)");
                println!("{:-<60}", "");
                
                let mut sorted: Vec<_> = latencies.iter().collect();
                sorted.sort_by(|a, b| a.1.partial_cmp(b.1).unwrap());
                
                for (route, latency) in sorted.iter().take(20) {
                    println!("{:<40} {:>10.2}", route, latency);
                }
            }
        }
    }
    
    Ok(())
}

fn parse_region(s: &str) -> Result<GeographicRegion> {
    match s.to_lowercase().as_str() {
        "na" | "northamerica" | "north_america" => Ok(GeographicRegion::NorthAmerica),
        "eu" | "europe" => Ok(GeographicRegion::Europe),
        "ap" | "asiapacific" | "asia_pacific" | "asia" => Ok(GeographicRegion::AsiaPacific),
        "sa" | "southamerica" | "south_america" => Ok(GeographicRegion::SouthAmerica),
        "af" | "africa" => Ok(GeographicRegion::Africa),
        "oc" | "oceania" => Ok(GeographicRegion::Oceania),
        "unknown" => Ok(GeographicRegion::Unknown),
        _ => Err(anyhow::anyhow!("Invalid region: {}. Valid regions: NA, EU, AP, SA, AF, OC", s))
    }
}