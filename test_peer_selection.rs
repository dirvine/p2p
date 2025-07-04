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
#!/usr/bin/env rust-script
//! Step 2: Testing Peer Selection Strategy for K=8 Replication
//! 
//! Run with: `rustc test_peer_selection.rs && ./test_peer_selection`

use std::time::{Duration, SystemTime};
use std::collections::HashMap;

// Re-use our foundation types from Step 1
pub type PeerId = String;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Key {
    hash: [u8; 32],
}

impl Key {
    pub fn from(data: Vec<u8>) -> Self {
        let mut hash = [0u8; 32];
        hash[..data.len().min(32)].copy_from_slice(&data[..data.len().min(32)]);
        Self { hash }
    }
    
    /// Get key as bytes for distance calculation
    pub fn as_bytes(&self) -> &[u8] {
        &self.hash
    }
    
    /// Calculate XOR distance between two keys (Kademlia distance metric)
    pub fn distance(&self, other: &Key) -> u64 {
        let mut xor_result = [0u8; 8];
        for i in 0..8 {
            xor_result[i] = self.hash[i] ^ other.hash[i];
        }
        u64::from_be_bytes(xor_result)
    }
}

#[derive(Debug, Clone)]
pub struct ReplicationConfig {
    pub replication_factor: usize,
    pub min_replication_factor: usize,
    pub preferred_distance_factor: f64,
    pub geographic_awareness: bool,
    pub repair_threshold: usize,
    pub repair_interval: Duration,
    pub max_repair_concurrent: usize,
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            replication_factor: 8,
            min_replication_factor: 3,
            preferred_distance_factor: 0.3,
            geographic_awareness: true,
            repair_threshold: 5,
            repair_interval: Duration::from_secs(300),
            max_repair_concurrent: 3,
        }
    }
}

/// Geographic information for peer distribution
#[derive(Debug, Clone)]
pub struct PeerGeographicInfo {
    pub peer_id: PeerId,
    pub region: String,           // Geographic region (e.g., "us-east", "eu-west")
    pub country_code: String,     // ISO country code
    pub latitude: Option<f64>,    // Approximate coordinates
    pub longitude: Option<f64>,
    pub network_provider: Option<String>, // ISP or cloud provider
    pub estimated_rtt: Option<Duration>,  // Round-trip time estimate
}

/// A peer candidate for replication
#[derive(Debug, Clone)]
pub struct PeerCandidate {
    pub peer_id: PeerId,
    pub distance: u64,           // XOR distance from target key
    pub geographic_info: Option<PeerGeographicInfo>,
    pub last_seen: SystemTime,
    pub is_online: bool,
}

/// Error types for peer selection
#[derive(Debug)]
pub enum PeerSelectionError {
    NoPeersAvailable,
    InsufficientPeers { required: usize, available: usize },
    GeographicInfoUnavailable,
}

/// Peer selection strategy implementation
#[derive(Debug)]
pub struct PeerSelectionStrategy {
    config: ReplicationConfig,
}

impl PeerSelectionStrategy {
    pub fn new(config: ReplicationConfig) -> Self {
        Self { config }
    }
    
    /// Select optimal peers for replication based on XOR distance and network topology
    pub fn select_optimal_peers(
        &self,
        target_key: &Key,
        available_peers: Vec<PeerCandidate>,
        target_count: usize,
    ) -> Result<Vec<PeerCandidate>, PeerSelectionError> {
        if available_peers.is_empty() {
            return Err(PeerSelectionError::NoPeersAvailable);
        }
        
        if available_peers.len() < self.config.min_replication_factor {
            return Err(PeerSelectionError::InsufficientPeers {
                required: self.config.min_replication_factor,
                available: available_peers.len(),
            });
        }
        
        // Step 1: Calculate distances and sort by proximity
        let mut peer_distances = self.calculate_peer_distances(target_key, available_peers);
        
        // Step 2: Apply geographic distribution if enabled
        if self.config.geographic_awareness {
            peer_distances = self.apply_geographic_distribution(peer_distances, target_count)?;
        }
        
        // Step 3: Select top K peers
        let selected_peers = peer_distances
            .into_iter()
            .take(target_count)
            .collect();
        
        Ok(selected_peers)
    }
    
    /// Calculate XOR distances for all peers and sort by proximity
    fn calculate_peer_distances(
        &self,
        target_key: &Key,
        peers: Vec<PeerCandidate>,
    ) -> Vec<PeerCandidate> {
        let mut peer_distances: Vec<PeerCandidate> = peers
            .into_iter()
            .map(|mut peer| {
                let peer_key = Key::from(peer.peer_id.as_bytes().to_vec());
                peer.distance = target_key.distance(&peer_key);
                peer
            })
            .collect();
        
        // Sort by XOR distance (closest first)
        peer_distances.sort_by_key(|peer| peer.distance);
        
        peer_distances
    }
    
    /// Apply geographic distribution to improve fault tolerance
    fn apply_geographic_distribution(
        &self,
        peer_distances: Vec<PeerCandidate>,
        target_count: usize,
    ) -> Result<Vec<PeerCandidate>, PeerSelectionError> {
        // Group peers by geographic region
        let mut regions: HashMap<String, Vec<PeerCandidate>> = HashMap::new();
        
        for peer in peer_distances {
            let region = peer.geographic_info
                .as_ref()
                .map(|info| info.region.clone())
                .unwrap_or_else(|| "unknown".to_string());
            
            regions.entry(region).or_default().push(peer);
        }
        
        // Select peers from different regions when possible
        let mut result = Vec::new();
        let mut region_order: Vec<_> = regions.keys().cloned().collect();
        region_order.sort(); // Deterministic ordering
        
        while result.len() < target_count {
            let mut added_any = false;
            
            for region in &region_order {
                if let Some(region_peers) = regions.get_mut(region) {
                    if !region_peers.is_empty() {
                        let peer = region_peers.remove(0);
                        result.push(peer);
                        added_any = true;
                        
                        if result.len() >= target_count {
                            break;
                        }
                    }
                }
            }
            
            if !added_any {
                break; // No more peers available
            }
            
            // Remove empty regions
            regions.retain(|_, peers| !peers.is_empty());
            region_order.retain(|region| regions.contains_key(region));
        }
        
        Ok(result)
    }
    
    /// Select peers with good health scores
    pub fn filter_healthy_peers(
        &self,
        peers: Vec<PeerCandidate>,
        min_last_seen: Duration,
    ) -> Vec<PeerCandidate> {
        let cutoff_time = SystemTime::now() - min_last_seen;
        
        peers.into_iter()
            .filter(|peer| peer.is_online && peer.last_seen >= cutoff_time)
            .collect()
    }
    
    /// Create a balanced selection considering both distance and diversity
    pub fn balanced_selection(
        &self,
        target_key: &Key,
        available_peers: Vec<PeerCandidate>,
        target_count: usize,
    ) -> Result<Vec<PeerCandidate>, PeerSelectionError> {
        // Filter for healthy peers first
        let healthy_peers = self.filter_healthy_peers(available_peers, Duration::from_secs(300));
        
        if healthy_peers.is_empty() {
            return Err(PeerSelectionError::NoPeersAvailable);
        }
        
        // Use the main selection algorithm
        self.select_optimal_peers(target_key, healthy_peers, target_count)
    }
}

// Test functions
fn create_test_peers() -> Vec<PeerCandidate> {
    vec![
        PeerCandidate {
            peer_id: "peer_1_us_east".to_string(),
            distance: 0, // Will be calculated
            geographic_info: Some(PeerGeographicInfo {
                peer_id: "peer_1_us_east".to_string(),
                region: "us-east".to_string(),
                country_code: "US".to_string(),
                latitude: Some(40.7128),
                longitude: Some(-74.0060),
                network_provider: Some("AWS".to_string()),
                estimated_rtt: Some(Duration::from_millis(50)),
            }),
            last_seen: SystemTime::now() - Duration::from_secs(10),
            is_online: true,
        },
        PeerCandidate {
            peer_id: "peer_2_us_west".to_string(),
            distance: 0,
            geographic_info: Some(PeerGeographicInfo {
                peer_id: "peer_2_us_west".to_string(),
                region: "us-west".to_string(),
                country_code: "US".to_string(),
                latitude: Some(37.7749),
                longitude: Some(-122.4194),
                network_provider: Some("GCP".to_string()),
                estimated_rtt: Some(Duration::from_millis(80)),
            }),
            last_seen: SystemTime::now() - Duration::from_secs(5),
            is_online: true,
        },
        PeerCandidate {
            peer_id: "peer_3_eu_west".to_string(),
            distance: 0,
            geographic_info: Some(PeerGeographicInfo {
                peer_id: "peer_3_eu_west".to_string(),
                region: "eu-west".to_string(),
                country_code: "IE".to_string(),
                latitude: Some(53.3498),
                longitude: Some(-6.2603),
                network_provider: Some("Azure".to_string()),
                estimated_rtt: Some(Duration::from_millis(120)),
            }),
            last_seen: SystemTime::now() - Duration::from_secs(20),
            is_online: true,
        },
        PeerCandidate {
            peer_id: "peer_4_ap_southeast".to_string(),
            distance: 0,
            geographic_info: Some(PeerGeographicInfo {
                peer_id: "peer_4_ap_southeast".to_string(),
                region: "ap-southeast".to_string(),
                country_code: "SG".to_string(),
                latitude: Some(1.3521),
                longitude: Some(103.8198),
                network_provider: Some("DigitalOcean".to_string()),
                estimated_rtt: Some(Duration::from_millis(200)),
            }),
            last_seen: SystemTime::now() - Duration::from_secs(15),
            is_online: true,
        },
        PeerCandidate {
            peer_id: "peer_5_us_east_2".to_string(),
            distance: 0,
            geographic_info: Some(PeerGeographicInfo {
                peer_id: "peer_5_us_east_2".to_string(),
                region: "us-east".to_string(),
                country_code: "US".to_string(),
                latitude: Some(40.7128),
                longitude: Some(-74.0060),
                network_provider: Some("Linode".to_string()),
                estimated_rtt: Some(Duration::from_millis(55)),
            }),
            last_seen: SystemTime::now() - Duration::from_secs(8),
            is_online: true,
        },
        PeerCandidate {
            peer_id: "peer_6_offline".to_string(),
            distance: 0,
            geographic_info: Some(PeerGeographicInfo {
                peer_id: "peer_6_offline".to_string(),
                region: "us-central".to_string(),
                country_code: "US".to_string(),
                latitude: Some(41.8781),
                longitude: Some(-87.6298),
                network_provider: Some("AWS".to_string()),
                estimated_rtt: Some(Duration::from_millis(70)),
            }),
            last_seen: SystemTime::now() - Duration::from_secs(500), // Old
            is_online: false,
        },
    ]
}

fn test_basic_peer_selection() {
    println!("Testing basic peer selection...");
    let config = ReplicationConfig::default();
    let strategy = PeerSelectionStrategy::new(config);
    let peers = create_test_peers();
    let target_key = Key::from(vec![1, 2, 3, 4, 5]);
    
    let result = strategy.select_optimal_peers(&target_key, peers, 4);
    
    match result {
        Ok(selected_peers) => {
            assert_eq!(selected_peers.len(), 4);
            println!("✓ Selected {} peers for replication", selected_peers.len());
            
            for peer in &selected_peers {
                println!("  - {} (region: {}, distance: {})", 
                    peer.peer_id,
                    peer.geographic_info.as_ref().map_or("unknown", |g| &g.region),
                    peer.distance
                );
            }
        }
        Err(e) => {
            panic!("Peer selection failed: {:?}", e);
        }
    }
}

fn test_geographic_distribution() {
    println!("\nTesting geographic distribution...");
    let mut config = ReplicationConfig::default();
    config.geographic_awareness = true;
    let strategy = PeerSelectionStrategy::new(config);
    let peers = create_test_peers();
    let target_key = Key::from(vec![10, 20, 30]);
    
    let result = strategy.select_optimal_peers(&target_key, peers, 4);
    
    match result {
        Ok(selected_peers) => {
            // Check that we have geographic diversity
            let regions: std::collections::HashSet<String> = selected_peers
                .iter()
                .filter_map(|p| p.geographic_info.as_ref().map(|g| g.region.clone()))
                .collect();
            
            println!("✓ Selected peers from {} different regions", regions.len());
            println!("  Regions: {:?}", regions);
            
            // Should have good geographic distribution
            assert!(regions.len() >= 3, "Should have peers from at least 3 regions");
        }
        Err(e) => {
            panic!("Geographic distribution test failed: {:?}", e);
        }
    }
}

fn test_healthy_peer_filtering() {
    println!("\nTesting healthy peer filtering...");
    let config = ReplicationConfig::default();
    let strategy = PeerSelectionStrategy::new(config);
    let peers = create_test_peers();
    
    // Filter with a 300 second threshold
    let healthy_peers = strategy.filter_healthy_peers(peers, Duration::from_secs(300));
    
    println!("✓ Filtered to {} healthy peers", healthy_peers.len());
    
    // Should exclude the offline peer and the very old peer
    assert_eq!(healthy_peers.len(), 5); // 6 total - 1 offline = 5
    
    // All remaining peers should be online and recent
    for peer in &healthy_peers {
        assert!(peer.is_online);
        let age = SystemTime::now().duration_since(peer.last_seen).unwrap();
        assert!(age < Duration::from_secs(300));
    }
}

fn test_balanced_selection() {
    println!("\nTesting balanced selection...");
    let config = ReplicationConfig::default();
    let strategy = PeerSelectionStrategy::new(config);
    let peers = create_test_peers();
    let target_key = Key::from(vec![50, 60, 70]);
    
    let result = strategy.balanced_selection(&target_key, peers, 3);
    
    match result {
        Ok(selected_peers) => {
            assert_eq!(selected_peers.len(), 3);
            println!("✓ Balanced selection returned {} peers", selected_peers.len());
            
            // Verify all selected peers are healthy
            for peer in &selected_peers {
                assert!(peer.is_online);
                let age = SystemTime::now().duration_since(peer.last_seen).unwrap();
                assert!(age < Duration::from_secs(300));
            }
            
            // Check geographic diversity
            let regions: std::collections::HashSet<String> = selected_peers
                .iter()
                .filter_map(|p| p.geographic_info.as_ref().map(|g| g.region.clone()))
                .collect();
            
            println!("  Geographic diversity: {} regions", regions.len());
        }
        Err(e) => {
            panic!("Balanced selection failed: {:?}", e);
        }
    }
}

fn test_distance_calculation() {
    println!("\nTesting XOR distance calculation...");
    let key1 = Key::from(vec![0, 0, 0, 0]);
    let key2 = Key::from(vec![1, 1, 1, 1]);
    let key3 = Key::from(vec![0, 0, 0, 0]);
    
    let distance1_2 = key1.distance(&key2);
    let distance1_3 = key1.distance(&key3);
    let distance2_3 = key2.distance(&key3);
    
    println!("✓ Distance calculations:");
    println!("  key1 to key2: {}", distance1_2);
    println!("  key1 to key3: {}", distance1_3);
    println!("  key2 to key3: {}", distance2_3);
    
    // Same keys should have distance 0
    assert_eq!(distance1_3, 0);
    
    // Different keys should have non-zero distance
    assert!(distance1_2 > 0);
    assert!(distance2_3 > 0);
    
    // Distance should be symmetric
    assert_eq!(key1.distance(&key2), key2.distance(&key1));
}

fn test_insufficient_peers_error() {
    println!("\nTesting insufficient peers error handling...");
    let mut config = ReplicationConfig::default();
    config.min_replication_factor = 10; // Set high requirement
    let strategy = PeerSelectionStrategy::new(config);
    let peers = create_test_peers(); // Only 6 peers
    let target_key = Key::from(vec![100, 200]);
    
    let result = strategy.select_optimal_peers(&target_key, peers, 8);
    
    match result {
        Err(PeerSelectionError::InsufficientPeers { required, available }) => {
            println!("✓ Correctly detected insufficient peers: required {}, available {}", required, available);
            assert_eq!(required, 10);
            assert_eq!(available, 6);
        }
        _ => {
            panic!("Expected InsufficientPeers error");
        }
    }
}

fn main() {
    println!("🧪 Running Peer Selection Strategy Tests\n");
    
    test_distance_calculation();
    test_basic_peer_selection();
    test_geographic_distribution();
    test_healthy_peer_filtering();
    test_balanced_selection();
    test_insufficient_peers_error();
    
    println!("\n🎉 All peer selection tests passed!");
    println!("✅ Step 2 Complete: Peer selection strategy is working correctly");
    
    println!("\n📋 Key Features Implemented:");
    println!("  ✓ XOR distance-based peer selection");
    println!("  ✓ Geographic diversity for fault tolerance");
    println!("  ✓ Health-based peer filtering");
    println!("  ✓ Balanced selection algorithm");
    println!("  ✓ Error handling for edge cases");
    
    println!("\n📋 Next Steps:");
    println!("  3. Add replication tracking system");
    println!("  4. Implement repair scheduler");
    println!("  5. Create enhanced record manager");
    println!("  6. Write integration tests");
}