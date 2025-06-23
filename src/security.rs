//! Security module
//!
//! This module provides cryptographic functionality and Sybil protection for the P2P network.
//! It implements IPv6-based node ID generation and IP diversity enforcement to prevent
//! large-scale Sybil attacks while maintaining network openness.

use crate::PeerId;
use anyhow::{anyhow, Result};
use ed25519_dalek::{Keypair, PublicKey, Signature, Signer, Verifier};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::net::Ipv6Addr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

/// IPv6-based node identity that binds node ID to actual network location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IPv6NodeID {
    /// Derived node ID (SHA256 of ipv6_addr + public_key + salt)
    pub node_id: Vec<u8>,
    /// IPv6 address this node ID is bound to
    pub ipv6_addr: Ipv6Addr,
    /// Ed25519 public key for signatures
    pub public_key: Vec<u8>,
    /// Signature proving ownership of the IPv6 address and keys
    pub signature: Vec<u8>,
    /// Timestamp when this ID was generated (seconds since epoch)
    pub timestamp_secs: u64,
    /// Salt used in node ID generation (for freshness)
    pub salt: Vec<u8>,
}

/// Configuration for IP diversity enforcement at multiple subnet levels
#[derive(Debug, Clone)]
pub struct IPDiversityConfig {
    /// Maximum nodes per /64 subnet (default: 1)
    pub max_nodes_per_64: usize,
    /// Maximum nodes per /48 allocation (default: 3)  
    pub max_nodes_per_48: usize,
    /// Maximum nodes per /32 region (default: 10)
    pub max_nodes_per_32: usize,
    /// Maximum nodes per AS number (default: 20)
    pub max_nodes_per_asn: usize,
    /// Enable GeoIP-based diversity checks
    pub enable_geolocation_check: bool,
    /// Minimum number of different countries required
    pub min_geographic_diversity: usize,
}

/// Analysis of an IPv6 address for diversity enforcement
#[derive(Debug, Clone)]
pub struct IPAnalysis {
    /// /64 subnet (host allocation)
    pub subnet_64: Ipv6Addr,
    /// /48 subnet (site allocation)
    pub subnet_48: Ipv6Addr,
    /// /32 subnet (ISP allocation)
    pub subnet_32: Ipv6Addr,
    /// Autonomous System Number (if available)
    pub asn: Option<u32>,
    /// Country code from GeoIP lookup
    pub country: Option<String>,
    /// Whether this is a known hosting/VPS provider
    pub is_hosting_provider: bool,
    /// Whether this is a known VPN provider
    pub is_vpn_provider: bool,
    /// Historical reputation score for this IP range
    pub reputation_score: f64,
}

/// Node reputation tracking for security-aware routing
#[derive(Debug, Clone)]
pub struct NodeReputation {
    /// Peer ID
    pub peer_id: PeerId,
    /// Fraction of queries answered successfully
    pub response_rate: f64,
    /// Average response time
    pub response_time: Duration,
    /// Consistency of provided data (0.0-1.0)
    pub consistency_score: f64,
    /// Estimated continuous uptime
    pub uptime_estimate: Duration,
    /// Accuracy of routing information provided
    pub routing_accuracy: f64,
    /// Last time this node was seen
    pub last_seen: SystemTime,
    /// Total number of interactions
    pub interaction_count: u64,
}

impl Default for IPDiversityConfig {
    fn default() -> Self {
        Self {
            max_nodes_per_64: 1,
            max_nodes_per_48: 3,
            max_nodes_per_32: 10,
            max_nodes_per_asn: 20,
            enable_geolocation_check: true,
            min_geographic_diversity: 3,
        }
    }
}

impl IPv6NodeID {
    /// Generate a new IPv6-based node ID
    pub fn generate(ipv6_addr: Ipv6Addr, keypair: &Keypair) -> Result<Self> {
        let mut rng = rand::thread_rng();
        let mut salt = vec![0u8; 16];
        rand::RngCore::fill_bytes(&mut rng, &mut salt);
        
        let timestamp = SystemTime::now();
        let timestamp_secs = timestamp.duration_since(UNIX_EPOCH)?.as_secs();
        let public_key = keypair.public.to_bytes().to_vec();
        
        // Generate node ID: SHA256(ipv6_address || public_key || salt || timestamp)
        let mut hasher = Sha256::new();
        hasher.update(ipv6_addr.octets());
        hasher.update(&public_key);
        hasher.update(&salt);
        hasher.update(&timestamp_secs.to_le_bytes());
        let node_id = hasher.finalize().to_vec();
        
        // Create signature proving ownership
        let mut message_to_sign = Vec::new();
        message_to_sign.extend_from_slice(&ipv6_addr.octets());
        message_to_sign.extend_from_slice(&public_key);
        message_to_sign.extend_from_slice(&salt);
        message_to_sign.extend_from_slice(&timestamp_secs.to_le_bytes());
        
        let signature = keypair.sign(&message_to_sign).to_bytes().to_vec();
        
        Ok(IPv6NodeID {
            node_id,
            ipv6_addr,
            public_key,
            signature,
            timestamp_secs,
            salt,
        })
    }
    
    /// Verify that this node ID is valid and properly signed
    pub fn verify(&self) -> Result<bool> {
        // Reconstruct the node ID
        let mut hasher = Sha256::new();
        hasher.update(self.ipv6_addr.octets());
        hasher.update(&self.public_key);
        hasher.update(&self.salt);
        hasher.update(&self.timestamp_secs.to_le_bytes());
        let expected_node_id = hasher.finalize();
        
        // Verify node ID matches
        if expected_node_id.as_slice() != &self.node_id {
            return Ok(false);
        }
        
        // Verify signature
        if self.public_key.len() != 32 {
            return Ok(false);
        }
        if self.signature.len() != 64 {
            return Ok(false);
        }
        
        let mut pk_bytes = [0u8; 32];
        pk_bytes.copy_from_slice(&self.public_key);
        let public_key = PublicKey::from_bytes(&pk_bytes)
            .map_err(|e| anyhow!("Invalid public key: {}", e))?;
            
        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(&self.signature);
        let signature = Signature::from_bytes(&sig_bytes)
            .map_err(|e| anyhow!("Invalid signature: {}", e))?;
        
        let mut message_to_verify = Vec::new();
        message_to_verify.extend_from_slice(&self.ipv6_addr.octets());
        message_to_verify.extend_from_slice(&self.public_key);
        message_to_verify.extend_from_slice(&self.salt);
        message_to_verify.extend_from_slice(&self.timestamp_secs.to_le_bytes());
        
        match public_key.verify(&message_to_verify, &signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    /// Extract /64 subnet from IPv6 address
    pub fn extract_subnet_64(&self) -> Ipv6Addr {
        let octets = self.ipv6_addr.octets();
        let mut subnet = [0u8; 16];
        subnet[..8].copy_from_slice(&octets[..8]); // Keep first 64 bits, zero the rest
        Ipv6Addr::from(subnet)
    }
    
    /// Extract /48 subnet from IPv6 address
    pub fn extract_subnet_48(&self) -> Ipv6Addr {
        let octets = self.ipv6_addr.octets();
        let mut subnet = [0u8; 16];
        subnet[..6].copy_from_slice(&octets[..6]); // Keep first 48 bits, zero the rest
        Ipv6Addr::from(subnet)
    }
    
    /// Extract /32 subnet from IPv6 address
    pub fn extract_subnet_32(&self) -> Ipv6Addr {
        let octets = self.ipv6_addr.octets();
        let mut subnet = [0u8; 16];
        subnet[..4].copy_from_slice(&octets[..4]); // Keep first 32 bits, zero the rest
        Ipv6Addr::from(subnet)
    }
}

/// IP diversity enforcement system
pub struct IPDiversityEnforcer {
    config: IPDiversityConfig,
    subnet_64_counts: HashMap<Ipv6Addr, usize>,
    subnet_48_counts: HashMap<Ipv6Addr, usize>,
    subnet_32_counts: HashMap<Ipv6Addr, usize>,
    asn_counts: HashMap<u32, usize>,
    country_counts: HashMap<String, usize>,
}

impl IPDiversityEnforcer {
    /// Create a new IP diversity enforcer
    pub fn new(config: IPDiversityConfig) -> Self {
        Self {
            config,
            subnet_64_counts: HashMap::new(),
            subnet_48_counts: HashMap::new(),
            subnet_32_counts: HashMap::new(),
            asn_counts: HashMap::new(),
            country_counts: HashMap::new(),
        }
    }
    
    /// Analyze an IPv6 address for diversity enforcement
    pub fn analyze_ip(&self, ipv6_addr: Ipv6Addr) -> Result<IPAnalysis> {
        let subnet_64 = Self::extract_subnet_prefix(ipv6_addr, 64);
        let subnet_48 = Self::extract_subnet_prefix(ipv6_addr, 48);
        let subnet_32 = Self::extract_subnet_prefix(ipv6_addr, 32);
        
        // TODO: Implement ASN lookup (requires external database)
        let asn = None;
        
        // TODO: Implement GeoIP lookup (requires external database)
        let country = None;
        
        // TODO: Implement hosting/VPN provider detection
        let is_hosting_provider = false;
        let is_vpn_provider = false;
        
        // Default reputation for new IPs
        let reputation_score = 0.5;
        
        Ok(IPAnalysis {
            subnet_64,
            subnet_48,
            subnet_32,
            asn,
            country,
            is_hosting_provider,
            is_vpn_provider,
            reputation_score,
        })
    }
    
    /// Check if a new node can be accepted based on IP diversity constraints
    pub fn can_accept_node(&self, ip_analysis: &IPAnalysis) -> bool {
        // Check /64 subnet limit
        if let Some(&count) = self.subnet_64_counts.get(&ip_analysis.subnet_64) {
            if count >= self.config.max_nodes_per_64 {
                return false;
            }
        }
        
        // Check /48 subnet limit
        if let Some(&count) = self.subnet_48_counts.get(&ip_analysis.subnet_48) {
            if count >= self.config.max_nodes_per_48 {
                return false;
            }
        }
        
        // Check /32 subnet limit
        if let Some(&count) = self.subnet_32_counts.get(&ip_analysis.subnet_32) {
            if count >= self.config.max_nodes_per_32 {
                return false;
            }
        }
        
        // Check ASN limit
        if let Some(asn) = ip_analysis.asn {
            if let Some(&count) = self.asn_counts.get(&asn) {
                if count >= self.config.max_nodes_per_asn {
                    return false;
                }
            }
        }
        
        // Stricter limits for hosting providers
        if ip_analysis.is_hosting_provider || ip_analysis.is_vpn_provider {
            // Reduce limits by half for hosting providers
            let hosting_64_limit = std::cmp::max(1, self.config.max_nodes_per_64 / 2);
            let hosting_48_limit = std::cmp::max(1, self.config.max_nodes_per_48 / 2);
            
            if let Some(&count) = self.subnet_64_counts.get(&ip_analysis.subnet_64) {
                if count >= hosting_64_limit {
                    return false;
                }
            }
            
            if let Some(&count) = self.subnet_48_counts.get(&ip_analysis.subnet_48) {
                if count >= hosting_48_limit {
                    return false;
                }
            }
        }
        
        true
    }
    
    /// Add a node to the diversity tracking
    pub fn add_node(&mut self, ip_analysis: &IPAnalysis) -> Result<()> {
        if !self.can_accept_node(ip_analysis) {
            return Err(anyhow!("IP diversity limits exceeded"));
        }
        
        // Update counts
        *self.subnet_64_counts.entry(ip_analysis.subnet_64).or_insert(0) += 1;
        *self.subnet_48_counts.entry(ip_analysis.subnet_48).or_insert(0) += 1;
        *self.subnet_32_counts.entry(ip_analysis.subnet_32).or_insert(0) += 1;
        
        if let Some(asn) = ip_analysis.asn {
            *self.asn_counts.entry(asn).or_insert(0) += 1;
        }
        
        if let Some(ref country) = ip_analysis.country {
            *self.country_counts.entry(country.clone()).or_insert(0) += 1;
        }
        
        Ok(())
    }
    
    /// Remove a node from diversity tracking
    pub fn remove_node(&mut self, ip_analysis: &IPAnalysis) {
        if let Some(count) = self.subnet_64_counts.get_mut(&ip_analysis.subnet_64) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.subnet_64_counts.remove(&ip_analysis.subnet_64);
            }
        }
        
        if let Some(count) = self.subnet_48_counts.get_mut(&ip_analysis.subnet_48) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.subnet_48_counts.remove(&ip_analysis.subnet_48);
            }
        }
        
        if let Some(count) = self.subnet_32_counts.get_mut(&ip_analysis.subnet_32) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.subnet_32_counts.remove(&ip_analysis.subnet_32);
            }
        }
        
        if let Some(asn) = ip_analysis.asn {
            if let Some(count) = self.asn_counts.get_mut(&asn) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    self.asn_counts.remove(&asn);
                }
            }
        }
        
        if let Some(ref country) = ip_analysis.country {
            if let Some(count) = self.country_counts.get_mut(country) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    self.country_counts.remove(country);
                }
            }
        }
    }
    
    /// Extract network prefix of specified length from IPv6 address
    pub fn extract_subnet_prefix(addr: Ipv6Addr, prefix_len: u8) -> Ipv6Addr {
        let octets = addr.octets();
        let mut subnet = [0u8; 16];
        
        let bytes_to_copy = (prefix_len / 8) as usize;
        let remaining_bits = prefix_len % 8;
        
        // Copy full bytes
        if bytes_to_copy < 16 {
            subnet[..bytes_to_copy].copy_from_slice(&octets[..bytes_to_copy]);
        } else {
            subnet.copy_from_slice(&octets);
        }
        
        // Handle partial byte
        if remaining_bits > 0 && bytes_to_copy < 16 {
            let mask = 0xFF << (8 - remaining_bits);
            subnet[bytes_to_copy] = octets[bytes_to_copy] & mask;
        }
        
        Ipv6Addr::from(subnet)
    }
    
    /// Get diversity statistics
    pub fn get_diversity_stats(&self) -> DiversityStats {
        DiversityStats {
            total_64_subnets: self.subnet_64_counts.len(),
            total_48_subnets: self.subnet_48_counts.len(),
            total_32_subnets: self.subnet_32_counts.len(),
            total_asns: self.asn_counts.len(),
            total_countries: self.country_counts.len(),
            max_nodes_per_64: self.subnet_64_counts.values().max().copied().unwrap_or(0),
            max_nodes_per_48: self.subnet_48_counts.values().max().copied().unwrap_or(0),
            max_nodes_per_32: self.subnet_32_counts.values().max().copied().unwrap_or(0),
        }
    }
}

/// Diversity statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiversityStats {
    pub total_64_subnets: usize,
    pub total_48_subnets: usize,
    pub total_32_subnets: usize,
    pub total_asns: usize,
    pub total_countries: usize,
    pub max_nodes_per_64: usize,
    pub max_nodes_per_48: usize,
    pub max_nodes_per_32: usize,
}

/// Reputation manager for tracking node behavior
pub struct ReputationManager {
    reputations: HashMap<PeerId, NodeReputation>,
    reputation_decay: f64,
    min_reputation: f64,
}

impl ReputationManager {
    /// Create a new reputation manager
    pub fn new(reputation_decay: f64, min_reputation: f64) -> Self {
        Self {
            reputations: HashMap::new(),
            reputation_decay,
            min_reputation,
        }
    }
    
    /// Get reputation for a peer
    pub fn get_reputation(&self, peer_id: &PeerId) -> Option<&NodeReputation> {
        self.reputations.get(peer_id)
    }
    
    /// Update reputation based on interaction
    pub fn update_reputation(&mut self, peer_id: &PeerId, success: bool, response_time: Duration) {
        let reputation = self.reputations.entry(peer_id.clone()).or_insert_with(|| {
            NodeReputation {
                peer_id: peer_id.clone(),
                response_rate: 0.5,
                response_time: Duration::from_millis(500),
                consistency_score: 0.5,
                uptime_estimate: Duration::from_secs(0),
                routing_accuracy: 0.5,
                last_seen: SystemTime::now(),
                interaction_count: 0,
            }
        });
        
        // Update with exponential moving average
        let alpha = 0.1; // Learning rate
        
        if success {
            reputation.response_rate = reputation.response_rate * (1.0 - alpha) + alpha;
        } else {
            reputation.response_rate = reputation.response_rate * (1.0 - alpha);
        }
        
        // Update response time
        let response_time_ms = response_time.as_millis() as f64;
        let current_response_ms = reputation.response_time.as_millis() as f64;
        let new_response_ms = current_response_ms * (1.0 - alpha) + response_time_ms * alpha;
        reputation.response_time = Duration::from_millis(new_response_ms as u64);
        
        reputation.last_seen = SystemTime::now();
        reputation.interaction_count += 1;
    }
    
    /// Apply time-based reputation decay
    pub fn apply_decay(&mut self) {
        let now = SystemTime::now();
        
        self.reputations.retain(|_, reputation| {
            if let Ok(elapsed) = now.duration_since(reputation.last_seen) {
                // Decay reputation over time
                let decay_factor = (-elapsed.as_secs_f64() / 3600.0 * self.reputation_decay).exp();
                reputation.response_rate *= decay_factor;
                reputation.consistency_score *= decay_factor;
                reputation.routing_accuracy *= decay_factor;
                
                // Remove nodes with very low reputation
                reputation.response_rate > self.min_reputation / 10.0
            } else {
                true
            }
        });
    }
}

/// Legacy security types for compatibility
pub mod security_types {
    use super::*;
    
    /// Ed25519 key pair wrapper
    pub struct KeyPair {
        inner: Keypair,
    }
    
    impl KeyPair {
        /// Generate a new key pair
        pub fn generate() -> Self {
            let mut csprng = rand::rngs::OsRng {};
            let keypair = Keypair::generate(&mut csprng);
            KeyPair { inner: keypair }
        }
        
        /// Get the inner Ed25519 keypair
        pub fn inner(&self) -> &Keypair {
            &self.inner
        }
        
        /// Get public key bytes
        pub fn public_key_bytes(&self) -> [u8; 32] {
            self.inner.public.to_bytes()
        }
        
        /// Sign a message
        pub fn sign(&self, message: &[u8]) -> [u8; 64] {
            self.inner.sign(message).to_bytes()
        }
    }
}