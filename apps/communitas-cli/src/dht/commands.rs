// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// DHT CLI commands implementation

use clap::Subcommand;
use anyhow::Result;
use super::manager::DHTManager;

#[derive(Subcommand, Debug)]
pub enum DHTCommands {
    /// Store a value in the DHT
    Put {
        /// Key for the data
        key: String,
        /// Value to store (can be file path with @prefix)
        value: String,
        /// TTL in seconds
        #[arg(long, default_value = "86400")]
        ttl: u64,
        /// Enable encryption
        #[arg(long)]
        encrypt: bool,
    },
    
    /// Retrieve a value from the DHT
    Get {
        /// Key to retrieve
        key: String,
        /// Output file path (optional)
        #[arg(long)]
        output: Option<String>,
        /// Decrypt the value
        #[arg(long)]
        decrypt: bool,
    },
    
    /// Delete a value from the DHT
    Delete {
        /// Key to delete
        key: String,
    },
    
    /// List all stored keys
    List {
        /// Filter by prefix
        #[arg(long)]
        prefix: Option<String>,
        /// Limit number of results
        #[arg(long, default_value = "100")]
        limit: usize,
        /// Show detailed information
        #[arg(long)]
        detailed: bool,
    },
    
    /// Show DHT statistics
    Stats {
        /// Show detailed metrics
        #[arg(long)]
        detailed: bool,
        /// Output format (json, table)
        #[arg(long, default_value = "table")]
        format: String,
    },
    
    /// Find nodes closest to a key
    FindNode {
        /// Key or node ID to search for
        key: String,
        /// Number of closest nodes to return
        #[arg(long, default_value = "20")]
        count: usize,
    },
    
    /// Replicate data to other nodes
    Replicate {
        /// Key to replicate (or 'all' for all keys)
        key: String,
        /// Target replication factor
        #[arg(long)]
        factor: Option<usize>,
    },
    
    /// Verify data integrity
    Verify {
        /// Key to verify (or 'all' for all keys)
        key: String,
        /// Fix corrupted data if possible
        #[arg(long)]
        repair: bool,
    },
    
    /// Export DHT data
    Export {
        /// Output file path
        output: String,
        /// Export format (json, binary)
        #[arg(long, default_value = "json")]
        format: String,
        /// Include metadata
        #[arg(long)]
        metadata: bool,
    },
    
    /// Import DHT data
    Import {
        /// Input file path
        input: String,
        /// Overwrite existing keys
        #[arg(long)]
        overwrite: bool,
        /// Validate before import
        #[arg(long)]
        validate: bool,
    },
    
    /// Manage DHT buckets
    Buckets {
        /// Show bucket distribution
        #[arg(long)]
        show: bool,
        /// Refresh stale buckets
        #[arg(long)]
        refresh: bool,
        /// Compact buckets
        #[arg(long)]
        compact: bool,
    },
    
    /// Configure DHT parameters
    Config {
        /// Set replication factor
        #[arg(long)]
        replication: Option<usize>,
        /// Set storage capacity (MB)
        #[arg(long)]
        capacity: Option<usize>,
        /// Set record TTL (seconds)
        #[arg(long)]
        ttl: Option<u64>,
        /// Enable/disable geographic routing
        #[arg(long)]
        geographic: Option<bool>,
        /// Show current configuration
        #[arg(long)]
        show: bool,
    },
}

pub async fn execute_dht_command(manager: &mut DHTManager, command: DHTCommands) -> Result<()> {
    use DHTCommands::*;
    
    match command {
        Put { key, value, ttl, encrypt } => {
            let data = if value.starts_with('@') {
                // Read from file
                let path = &value[1..];
                tokio::fs::read(path).await?
            } else {
                value.into_bytes()
            };
            
            let result = if encrypt {
                manager.put_encrypted(&key, data, ttl).await?
            } else {
                manager.put(&key, data, ttl).await?
            };
            
            println!("✓ Stored key: {}", key);
            println!("  Hash: {:?}", result.hash);
            println!("  Size: {} bytes", result.size);
            println!("  Replicas: {}", result.replicas);
        }
        
        Get { key, output, decrypt } => {
            let data = if decrypt {
                manager.get_encrypted(&key).await?
            } else {
                manager.get(&key).await?
            };
            
            if let Some(data) = data {
                if let Some(output_path) = output {
                    tokio::fs::write(&output_path, &data).await?;
                    println!("✓ Saved to: {}", output_path);
                } else {
                    // Try to display as string, fallback to hex
                    match String::from_utf8(data.clone()) {
                        Ok(s) => println!("{}", s),
                        Err(_) => println!("Hex: {}", hex::encode(&data)),
                    }
                }
            } else {
                println!("Key not found: {}", key);
            }
        }
        
        Delete { key } => {
            let deleted = manager.delete(&key).await?;
            if deleted {
                println!("✓ Deleted key: {}", key);
            } else {
                println!("Key not found: {}", key);
            }
        }
        
        List { prefix, limit, detailed } => {
            let keys = manager.list_keys(prefix.as_deref(), limit).await?;
            
            if detailed {
                println!("DHT Storage Contents:");
                println!("{:-<60}", "");
                for (key, info) in keys {
                    println!("Key: {}", key);
                    println!("  Size: {} bytes", info.size);
                    println!("  Created: {}", info.created);
                    println!("  TTL: {} seconds", info.ttl);
                    println!("  Replicas: {}", info.replicas);
                    println!();
                }
            } else {
                for (key, _) in keys {
                    println!("{}", key);
                }
            }
        }
        
        Stats { detailed, format } => {
            let stats = manager.get_stats().await?;
            
            if format == "json" {
                println!("{}", serde_json::to_string_pretty(&stats)?);
            } else {
                println!("DHT Statistics:");
                println!("{:-<40}", "");
                println!("Total Records: {}", stats.total_records);
                println!("Storage Used: {:.2} MB", stats.storage_used_mb);
                println!("Get Requests: {}", stats.get_requests);
                println!("Put Requests: {}", stats.put_requests);
                println!("Lookup Requests: {}", stats.lookup_requests);
                println!("Replication Count: {}", stats.replication_count);
                println!("Avg Response Time: {:.2} ms", stats.avg_response_time_ms);
                println!("Cache Hit Rate: {:.2}%", stats.cache_hit_rate * 100.0);
                
                if detailed {
                    let detailed_stats = manager.get_detailed_stats().await?;
                    println!("\nDetailed Metrics:");
                    println!("{:-<40}", "");
                    for (metric, value) in detailed_stats {
                        println!("{}: {}", metric, value);
                    }
                }
            }
        }
        
        FindNode { key, count } => {
            let nodes = manager.find_closest_nodes(&key, count).await?;
            
            println!("Closest nodes to '{}':", key);
            println!("{:-<60}", "");
            for (i, node) in nodes.iter().enumerate() {
                println!("{}. Node ID: {}", i + 1, node.id);
                println!("   Address: {}", node.address);
                println!("   Distance: {}", node.distance);
                println!("   Latency: {} ms", node.latency_ms);
                println!();
            }
        }
        
        Replicate { key, factor } => {
            let result = if key == "all" {
                manager.replicate_all(factor).await?
            } else {
                manager.replicate_key(&key, factor).await?
            };
            
            println!("✓ Replication complete");
            println!("  Keys processed: {}", result.keys_processed);
            println!("  New replicas: {}", result.new_replicas);
            println!("  Failed: {}", result.failed);
        }
        
        Verify { key, repair } => {
            let result = if key == "all" {
                manager.verify_all(repair).await?
            } else {
                manager.verify_key(&key, repair).await?
            };
            
            println!("Verification Results:");
            println!("{:-<40}", "");
            println!("Valid: {}", result.valid);
            println!("Corrupted: {}", result.corrupted);
            println!("Missing: {}", result.missing);
            if repair {
                println!("Repaired: {}", result.repaired);
            }
        }
        
        Export { output, format, metadata } => {
            let count = manager.export_data(&output, &format, metadata).await?;
            println!("✓ Exported {} records to {}", count, output);
        }
        
        Import { input, overwrite, validate } => {
            if validate {
                let validation = manager.validate_import(&input).await?;
                println!("Import validation:");
                println!("  Valid records: {}", validation.valid);
                println!("  Invalid records: {}", validation.invalid);
                if validation.invalid > 0 {
                    println!("  Errors: {:?}", validation.errors);
                    return Ok(());
                }
            }
            
            let count = manager.import_data(&input, overwrite).await?;
            println!("✓ Imported {} records from {}", count, input);
        }
        
        Buckets { show, refresh, compact } => {
            if show {
                let buckets = manager.get_bucket_info().await?;
                println!("DHT Bucket Distribution:");
                println!("{:-<60}", "");
                for bucket in buckets {
                    println!("Bucket {}: {} nodes (distance: {})", 
                        bucket.index, bucket.node_count, bucket.distance_range);
                }
            }
            
            if refresh {
                let refreshed = manager.refresh_buckets().await?;
                println!("✓ Refreshed {} stale buckets", refreshed);
            }
            
            if compact {
                let compacted = manager.compact_buckets().await?;
                println!("✓ Compacted {} buckets", compacted);
            }
        }
        
        Config { replication, capacity, ttl, geographic, show } => {
            if show {
                let config = manager.get_config().await?;
                println!("DHT Configuration:");
                println!("{:-<40}", "");
                println!("Replication Factor: {}", config.replication_factor);
                println!("Storage Capacity: {} MB", config.storage_capacity_mb);
                println!("Record TTL: {} seconds", config.record_ttl.as_secs());
                println!("Geographic Routing: {}", config.geographic_routing);
                println!("Auto Rebalance: {}", config.auto_rebalance);
                println!("Max Concurrent Ops: {}", config.max_concurrent_ops);
            } else {
                let mut updated = false;
                
                if let Some(r) = replication {
                    manager.set_replication_factor(r).await?;
                    println!("✓ Set replication factor to {}", r);
                    updated = true;
                }
                
                if let Some(c) = capacity {
                    manager.set_storage_capacity(c).await?;
                    println!("✓ Set storage capacity to {} MB", c);
                    updated = true;
                }
                
                if let Some(t) = ttl {
                    manager.set_default_ttl(t).await?;
                    println!("✓ Set default TTL to {} seconds", t);
                    updated = true;
                }
                
                if let Some(g) = geographic {
                    manager.set_geographic_routing(g).await?;
                    println!("✓ Geographic routing {}", if g { "enabled" } else { "disabled" });
                    updated = true;
                }
                
                if !updated {
                    println!("No configuration changes specified");
                }
            }
        }
    }
    
    Ok(())
}