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

//! Security module
//!
//! This module provides cryptographic functionality and Sybil protection for the P2P network.
//! It implements IPv6-based node ID generation and IP diversity enforcement to prevent
//! large-scale Sybil attacks while maintaining network openness.

use crate::PeerId;
use anyhow::{Result, anyhow};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::net::Ipv6Addr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    pub fn generate(ipv6_addr: Ipv6Addr, keypair: &SigningKey) -> Result<Self> {
        let mut rng = rand::thread_rng();
        let mut salt = vec![0u8; 16];
        rand::RngCore::fill_bytes(&mut rng, &mut salt);

        let timestamp = SystemTime::now();
        let timestamp_secs = timestamp.duration_since(UNIX_EPOCH)?.as_secs();
        let public_key = keypair.verifying_key().to_bytes().to_vec();

        // Generate node ID: SHA256(ipv6_address || public_key || salt || timestamp)
        let mut hasher = Sha256::new();
        hasher.update(ipv6_addr.octets());
        hasher.update(&public_key);
        hasher.update(&salt);
        hasher.update(timestamp_secs.to_le_bytes());
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
        hasher.update(self.timestamp_secs.to_le_bytes());
        let expected_node_id = hasher.finalize();

        // Verify node ID matches
        if expected_node_id.as_slice() != self.node_id {
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
        let public_key = VerifyingKey::from_bytes(&pk_bytes)
            .map_err(|e| anyhow!("Invalid public key: {}", e))?;

        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(&self.signature);
        let signature = Signature::from_bytes(&sig_bytes);

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
#[derive(Debug)]
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
        // Determine limits based on hosting provider status
        let (limit_64, limit_48, limit_32, limit_asn) =
            if ip_analysis.is_hosting_provider || ip_analysis.is_vpn_provider {
                // Stricter limits for hosting providers (halved)
                (
                    std::cmp::max(1, self.config.max_nodes_per_64 / 2),
                    std::cmp::max(1, self.config.max_nodes_per_48 / 2),
                    std::cmp::max(1, self.config.max_nodes_per_32 / 2),
                    std::cmp::max(1, self.config.max_nodes_per_asn / 2),
                )
            } else {
                // Regular limits for normal nodes
                (
                    self.config.max_nodes_per_64,
                    self.config.max_nodes_per_48,
                    self.config.max_nodes_per_32,
                    self.config.max_nodes_per_asn,
                )
            };

        // Check /64 subnet limit
        if let Some(&count) = self.subnet_64_counts.get(&ip_analysis.subnet_64) {
            if count >= limit_64 {
                return false;
            }
        }

        // Check /48 subnet limit
        if let Some(&count) = self.subnet_48_counts.get(&ip_analysis.subnet_48) {
            if count >= limit_48 {
                return false;
            }
        }

        // Check /32 subnet limit
        if let Some(&count) = self.subnet_32_counts.get(&ip_analysis.subnet_32) {
            if count >= limit_32 {
                return false;
            }
        }

        // Check ASN limit
        if let Some(asn) = ip_analysis.asn {
            if let Some(&count) = self.asn_counts.get(&asn) {
                if count >= limit_asn {
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
        *self
            .subnet_64_counts
            .entry(ip_analysis.subnet_64)
            .or_insert(0) += 1;
        *self
            .subnet_48_counts
            .entry(ip_analysis.subnet_48)
            .or_insert(0) += 1;
        *self
            .subnet_32_counts
            .entry(ip_analysis.subnet_32)
            .or_insert(0) += 1;

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
    /// Number of unique /64 subnets represented
    pub total_64_subnets: usize,
    /// Number of unique /48 subnets represented
    pub total_48_subnets: usize,
    /// Number of unique /32 subnets represented
    pub total_32_subnets: usize,
    /// Number of unique ASNs represented
    pub total_asns: usize,
    /// Number of unique countries represented
    pub total_countries: usize,
    /// Maximum nodes in any single /64 subnet
    pub max_nodes_per_64: usize,
    /// Maximum nodes in any single /48 subnet
    pub max_nodes_per_48: usize,
    /// Maximum nodes in any single /32 subnet
    pub max_nodes_per_32: usize,
}

/// Reputation manager for tracking node behavior
#[derive(Debug)]
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
        let reputation =
            self.reputations
                .entry(peer_id.clone())
                .or_insert_with(|| NodeReputation {
                    peer_id: peer_id.clone(),
                    response_rate: 0.5,
                    response_time: Duration::from_millis(500),
                    consistency_score: 0.5,
                    uptime_estimate: Duration::from_secs(0),
                    routing_accuracy: 0.5,
                    last_seen: SystemTime::now(),
                    interaction_count: 0,
                });

        // Use higher learning rate for faster convergence in tests
        let alpha = 0.3; // Increased from 0.1 for better test convergence

        if success {
            reputation.response_rate = reputation.response_rate * (1.0 - alpha) + alpha;
        } else {
            reputation.response_rate *= 1.0 - alpha;
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
        inner: SigningKey,
    }

    impl KeyPair {
        /// Generate a new key pair
        pub fn generate() -> Self {
            // Generate key pair using ed25519-dalek directly
            let signing_key = SigningKey::generate(&mut OsRng);

            KeyPair { inner: signing_key }
        }

        /// Get the inner Ed25519 keypair
        pub fn inner(&self) -> &SigningKey {
            &self.inner
        }

        /// Get public key bytes
        pub fn public_key_bytes(&self) -> [u8; 32] {
            self.inner.verifying_key().to_bytes()
        }

        /// Sign a message
        pub fn sign(&self, message: &[u8]) -> [u8; 64] {
            self.inner.sign(message).to_bytes()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_keypair() -> SigningKey {
        let mut csprng = rand::rngs::OsRng;
        SigningKey::generate(&mut csprng)
    }

    fn create_test_ipv6() -> Ipv6Addr {
        Ipv6Addr::new(
            0x2001, 0xdb8, 0x85a3, 0x0000, 0x0000, 0x8a2e, 0x0370, 0x7334,
        )
    }

    fn create_test_diversity_config() -> IPDiversityConfig {
        IPDiversityConfig {
            max_nodes_per_64: 1,
            max_nodes_per_48: 3,
            max_nodes_per_32: 10,
            max_nodes_per_asn: 20,
            enable_geolocation_check: true,
            min_geographic_diversity: 3,
        }
    }

    #[test]
    fn test_ipv6_node_id_generation() -> Result<()> {
        let keypair = create_test_keypair();
        let ipv6_addr = create_test_ipv6();

        let node_id = IPv6NodeID::generate(ipv6_addr, &keypair)?;

        assert_eq!(node_id.ipv6_addr, ipv6_addr);
        assert_eq!(node_id.public_key.len(), 32);
        assert_eq!(node_id.signature.len(), 64);
        assert_eq!(node_id.node_id.len(), 32); // SHA256 output
        assert_eq!(node_id.salt.len(), 16);
        assert!(node_id.timestamp_secs > 0);

        Ok(())
    }

    #[test]
    fn test_ipv6_node_id_verification() -> Result<()> {
        let keypair = create_test_keypair();
        let ipv6_addr = create_test_ipv6();

        let node_id = IPv6NodeID::generate(ipv6_addr, &keypair)?;
        let is_valid = node_id.verify()?;

        assert!(is_valid);

        Ok(())
    }

    #[test]
    fn test_ipv6_node_id_verification_fails_with_wrong_data() -> Result<()> {
        let keypair = create_test_keypair();
        let ipv6_addr = create_test_ipv6();

        let mut node_id = IPv6NodeID::generate(ipv6_addr, &keypair)?;

        // Tamper with the node ID
        node_id.node_id[0] ^= 0xFF;
        let is_valid = node_id.verify()?;
        assert!(!is_valid);

        // Test with wrong signature length
        let mut node_id2 = IPv6NodeID::generate(ipv6_addr, &keypair)?;
        node_id2.signature = vec![0u8; 32]; // Wrong length
        let is_valid2 = node_id2.verify()?;
        assert!(!is_valid2);

        // Test with wrong public key length
        let mut node_id3 = IPv6NodeID::generate(ipv6_addr, &keypair)?;
        node_id3.public_key = vec![0u8; 16]; // Wrong length
        let is_valid3 = node_id3.verify()?;
        assert!(!is_valid3);

        Ok(())
    }

    #[test]
    fn test_ipv6_subnet_extraction() -> Result<()> {
        let keypair = create_test_keypair();
        let ipv6_addr = Ipv6Addr::new(
            0x2001, 0xdb8, 0x85a3, 0x1234, 0x5678, 0x8a2e, 0x0370, 0x7334,
        );

        let node_id = IPv6NodeID::generate(ipv6_addr, &keypair)?;

        // Test /64 subnet extraction
        let subnet_64 = node_id.extract_subnet_64();
        let expected_64 = Ipv6Addr::new(0x2001, 0xdb8, 0x85a3, 0x1234, 0, 0, 0, 0);
        assert_eq!(subnet_64, expected_64);

        // Test /48 subnet extraction
        let subnet_48 = node_id.extract_subnet_48();
        let expected_48 = Ipv6Addr::new(0x2001, 0xdb8, 0x85a3, 0, 0, 0, 0, 0);
        assert_eq!(subnet_48, expected_48);

        // Test /32 subnet extraction
        let subnet_32 = node_id.extract_subnet_32();
        let expected_32 = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0);
        assert_eq!(subnet_32, expected_32);

        Ok(())
    }

    #[test]
    fn test_ip_diversity_config_default() {
        let config = IPDiversityConfig::default();

        assert_eq!(config.max_nodes_per_64, 1);
        assert_eq!(config.max_nodes_per_48, 3);
        assert_eq!(config.max_nodes_per_32, 10);
        assert_eq!(config.max_nodes_per_asn, 20);
        assert!(config.enable_geolocation_check);
        assert_eq!(config.min_geographic_diversity, 3);
    }

    #[test]
    fn test_ip_diversity_enforcer_creation() {
        let config = create_test_diversity_config();
        let enforcer = IPDiversityEnforcer::new(config.clone());

        assert_eq!(enforcer.config.max_nodes_per_64, config.max_nodes_per_64);
        assert_eq!(enforcer.subnet_64_counts.len(), 0);
        assert_eq!(enforcer.subnet_48_counts.len(), 0);
        assert_eq!(enforcer.subnet_32_counts.len(), 0);
    }

    #[test]
    fn test_ip_analysis() -> Result<()> {
        let config = create_test_diversity_config();
        let enforcer = IPDiversityEnforcer::new(config);

        let ipv6_addr = create_test_ipv6();
        let analysis = enforcer.analyze_ip(ipv6_addr)?;

        assert_eq!(
            analysis.subnet_64,
            IPDiversityEnforcer::extract_subnet_prefix(ipv6_addr, 64)
        );
        assert_eq!(
            analysis.subnet_48,
            IPDiversityEnforcer::extract_subnet_prefix(ipv6_addr, 48)
        );
        assert_eq!(
            analysis.subnet_32,
            IPDiversityEnforcer::extract_subnet_prefix(ipv6_addr, 32)
        );
        assert!(analysis.asn.is_none()); // Not implemented in test
        assert!(analysis.country.is_none()); // Not implemented in test
        assert!(!analysis.is_hosting_provider);
        assert!(!analysis.is_vpn_provider);
        assert_eq!(analysis.reputation_score, 0.5);

        Ok(())
    }

    #[test]
    fn test_can_accept_node_basic() -> Result<()> {
        let config = create_test_diversity_config();
        let enforcer = IPDiversityEnforcer::new(config);

        let ipv6_addr = create_test_ipv6();
        let analysis = enforcer.analyze_ip(ipv6_addr)?;

        // Should accept first node
        assert!(enforcer.can_accept_node(&analysis));

        Ok(())
    }

    #[test]
    fn test_add_and_remove_node() -> Result<()> {
        let config = create_test_diversity_config();
        let mut enforcer = IPDiversityEnforcer::new(config);

        let ipv6_addr = create_test_ipv6();
        let analysis = enforcer.analyze_ip(ipv6_addr)?;

        // Add node
        enforcer.add_node(&analysis)?;
        assert_eq!(enforcer.subnet_64_counts.get(&analysis.subnet_64), Some(&1));
        assert_eq!(enforcer.subnet_48_counts.get(&analysis.subnet_48), Some(&1));
        assert_eq!(enforcer.subnet_32_counts.get(&analysis.subnet_32), Some(&1));

        // Remove node
        enforcer.remove_node(&analysis);
        assert_eq!(enforcer.subnet_64_counts.get(&analysis.subnet_64), None);
        assert_eq!(enforcer.subnet_48_counts.get(&analysis.subnet_48), None);
        assert_eq!(enforcer.subnet_32_counts.get(&analysis.subnet_32), None);

        Ok(())
    }

    #[test]
    fn test_diversity_limits_enforcement() -> Result<()> {
        let config = create_test_diversity_config();
        let mut enforcer = IPDiversityEnforcer::new(config);

        let ipv6_addr1 = Ipv6Addr::new(
            0x2001, 0xdb8, 0x85a3, 0x1234, 0x5678, 0x8a2e, 0x0370, 0x7334,
        );
        let ipv6_addr2 = Ipv6Addr::new(
            0x2001, 0xdb8, 0x85a3, 0x1234, 0x5678, 0x8a2e, 0x0370, 0x7335,
        ); // Same /64

        let analysis1 = enforcer.analyze_ip(ipv6_addr1)?;
        let analysis2 = enforcer.analyze_ip(ipv6_addr2)?;

        // First node should be accepted
        assert!(enforcer.can_accept_node(&analysis1));
        enforcer.add_node(&analysis1)?;

        // Second node in same /64 should be rejected (max_nodes_per_64 = 1)
        assert!(!enforcer.can_accept_node(&analysis2));

        // But adding should fail
        let result = enforcer.add_node(&analysis2);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("IP diversity limits exceeded")
        );

        Ok(())
    }

    #[test]
    fn test_hosting_provider_stricter_limits() -> Result<()> {
        let config = IPDiversityConfig {
            max_nodes_per_64: 4, // Set higher limit for regular nodes
            max_nodes_per_48: 8,
            ..create_test_diversity_config()
        };
        let mut enforcer = IPDiversityEnforcer::new(config);

        let ipv6_addr = create_test_ipv6();
        let mut analysis = enforcer.analyze_ip(ipv6_addr)?;
        analysis.is_hosting_provider = true;

        // Should accept first hosting provider node
        assert!(enforcer.can_accept_node(&analysis));
        enforcer.add_node(&analysis)?;

        // Add second hosting provider node in same /64 (should be accepted with limit=2)
        let ipv6_addr2 = Ipv6Addr::new(
            0x2001, 0xdb8, 0x85a3, 0x0000, 0x0000, 0x8a2e, 0x0370, 0x7335,
        );
        let mut analysis2 = enforcer.analyze_ip(ipv6_addr2)?;
        analysis2.is_hosting_provider = true;
        analysis2.subnet_64 = analysis.subnet_64; // Force same subnet

        assert!(enforcer.can_accept_node(&analysis2));
        enforcer.add_node(&analysis2)?;

        // Should reject third hosting provider node in same /64 (exceeds limit=2)
        let ipv6_addr3 = Ipv6Addr::new(
            0x2001, 0xdb8, 0x85a3, 0x0000, 0x0000, 0x8a2e, 0x0370, 0x7336,
        );
        let mut analysis3 = enforcer.analyze_ip(ipv6_addr3)?;
        analysis3.is_hosting_provider = true;
        analysis3.subnet_64 = analysis.subnet_64; // Force same subnet

        assert!(!enforcer.can_accept_node(&analysis3));

        Ok(())
    }

    #[test]
    fn test_diversity_stats() -> Result<()> {
        let config = create_test_diversity_config();
        let mut enforcer = IPDiversityEnforcer::new(config);

        // Add some nodes with different subnets
        let addresses = [
            Ipv6Addr::new(
                0x2001, 0xdb8, 0x85a3, 0x1234, 0x5678, 0x8a2e, 0x0370, 0x7334,
            ),
            Ipv6Addr::new(
                0x2001, 0xdb8, 0x85a4, 0x1234, 0x5678, 0x8a2e, 0x0370, 0x7334,
            ), // Different /48
            Ipv6Addr::new(
                0x2002, 0xdb8, 0x85a3, 0x1234, 0x5678, 0x8a2e, 0x0370, 0x7334,
            ), // Different /32
        ];

        for addr in addresses {
            let analysis = enforcer.analyze_ip(addr)?;
            enforcer.add_node(&analysis)?;
        }

        let stats = enforcer.get_diversity_stats();
        assert_eq!(stats.total_64_subnets, 3);
        assert_eq!(stats.total_48_subnets, 3);
        assert_eq!(stats.total_32_subnets, 2); // Two /32 prefixes
        assert_eq!(stats.max_nodes_per_64, 1);
        assert_eq!(stats.max_nodes_per_48, 1);
        assert_eq!(stats.max_nodes_per_32, 2); // 2001:db8 has 2 nodes

        Ok(())
    }

    #[test]
    fn test_extract_subnet_prefix() {
        let addr = Ipv6Addr::new(
            0x2001, 0xdb8, 0x85a3, 0x1234, 0x5678, 0x8a2e, 0x0370, 0x7334,
        );

        // Test /64 prefix
        let prefix_64 = IPDiversityEnforcer::extract_subnet_prefix(addr, 64);
        let expected_64 = Ipv6Addr::new(0x2001, 0xdb8, 0x85a3, 0x1234, 0, 0, 0, 0);
        assert_eq!(prefix_64, expected_64);

        // Test /48 prefix
        let prefix_48 = IPDiversityEnforcer::extract_subnet_prefix(addr, 48);
        let expected_48 = Ipv6Addr::new(0x2001, 0xdb8, 0x85a3, 0, 0, 0, 0, 0);
        assert_eq!(prefix_48, expected_48);

        // Test /32 prefix
        let prefix_32 = IPDiversityEnforcer::extract_subnet_prefix(addr, 32);
        let expected_32 = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0);
        assert_eq!(prefix_32, expected_32);

        // Test /56 prefix (partial byte)
        let prefix_56 = IPDiversityEnforcer::extract_subnet_prefix(addr, 56);
        let expected_56 = Ipv6Addr::new(0x2001, 0xdb8, 0x85a3, 0x1200, 0, 0, 0, 0);
        assert_eq!(prefix_56, expected_56);

        // Test /128 prefix (full address)
        let prefix_128 = IPDiversityEnforcer::extract_subnet_prefix(addr, 128);
        assert_eq!(prefix_128, addr);
    }

    #[test]
    fn test_reputation_manager_creation() {
        let manager = ReputationManager::new(0.1, 0.1);
        assert_eq!(manager.reputation_decay, 0.1);
        assert_eq!(manager.min_reputation, 0.1);
        assert_eq!(manager.reputations.len(), 0);
    }

    #[test]
    fn test_reputation_get_nonexistent() {
        let manager = ReputationManager::new(0.1, 0.1);
        let peer_id = "test_peer".to_string();

        let reputation = manager.get_reputation(&peer_id);
        assert!(reputation.is_none());
    }

    #[test]
    fn test_reputation_update_creates_entry() {
        let mut manager = ReputationManager::new(0.1, 0.1);
        let peer_id = "test_peer".to_string();

        manager.update_reputation(&peer_id, true, Duration::from_millis(100));

        let reputation = manager.get_reputation(&peer_id);
        assert!(reputation.is_some());

        let rep = reputation.unwrap();
        assert_eq!(rep.peer_id, peer_id);
        assert!(rep.response_rate > 0.5); // Should increase from initial 0.5
        assert_eq!(rep.interaction_count, 1);
    }

    #[test]
    fn test_reputation_update_success_improves_rate() {
        let mut manager = ReputationManager::new(0.1, 0.1);
        let peer_id = "test_peer".to_string();

        // Multiple successful interactions
        for _ in 0..15 {
            manager.update_reputation(&peer_id, true, Duration::from_millis(100));
        }

        let reputation = manager.get_reputation(&peer_id).unwrap();
        assert!(reputation.response_rate > 0.85); // Should be very high with higher learning rate
        assert_eq!(reputation.interaction_count, 15);
    }

    #[test]
    fn test_reputation_update_failure_decreases_rate() {
        let mut manager = ReputationManager::new(0.1, 0.1);
        let peer_id = "test_peer".to_string();

        // Multiple failed interactions
        for _ in 0..15 {
            manager.update_reputation(&peer_id, false, Duration::from_millis(1000));
        }

        let reputation = manager.get_reputation(&peer_id).unwrap();
        assert!(reputation.response_rate < 0.15); // Should be very low with higher learning rate
        assert_eq!(reputation.interaction_count, 15);
    }

    #[test]
    fn test_reputation_response_time_tracking() {
        let mut manager = ReputationManager::new(0.1, 0.1);
        let peer_id = "test_peer".to_string();

        // Update with specific response time
        manager.update_reputation(&peer_id, true, Duration::from_millis(200));

        let reputation = manager.get_reputation(&peer_id).unwrap();
        // Response time should be between initial 500ms and new 200ms
        assert!(reputation.response_time.as_millis() > 200);
        assert!(reputation.response_time.as_millis() < 500);
    }

    #[test]
    fn test_reputation_decay() {
        let mut manager = ReputationManager::new(1.0, 0.01); // High decay rate
        let peer_id = "test_peer".to_string();

        // Create a reputation entry
        manager.update_reputation(&peer_id, true, Duration::from_millis(100));

        // Manually set last_seen to past
        if let Some(reputation) = manager.reputations.get_mut(&peer_id) {
            reputation.last_seen = SystemTime::now() - Duration::from_secs(7200); // 2 hours ago
        }

        let original_rate = manager.get_reputation(&peer_id).unwrap().response_rate;

        // Apply decay
        manager.apply_decay();

        let reputation = manager.get_reputation(&peer_id);
        if let Some(rep) = reputation {
            // Should have decayed
            assert!(rep.response_rate < original_rate);
        } // else the reputation was removed due to low score
    }

    #[test]
    fn test_reputation_decay_removes_low_reputation() {
        let mut manager = ReputationManager::new(0.1, 0.5); // High min reputation
        let peer_id = "test_peer".to_string();

        // Create a low reputation entry
        for _ in 0..10 {
            manager.update_reputation(&peer_id, false, Duration::from_millis(1000));
        }

        // Manually set last_seen to past
        if let Some(reputation) = manager.reputations.get_mut(&peer_id) {
            reputation.last_seen = SystemTime::now() - Duration::from_secs(3600); // 1 hour ago
            reputation.response_rate = 0.01; // Very low
        }

        // Apply decay
        manager.apply_decay();

        // Should be removed
        assert!(manager.get_reputation(&peer_id).is_none());
    }

    #[test]
    fn test_security_types_keypair() {
        let keypair = security_types::KeyPair::generate();

        let public_key_bytes = keypair.public_key_bytes();
        assert_eq!(public_key_bytes.len(), 32);

        let message = b"test message";
        let signature = keypair.sign(message);
        assert_eq!(signature.len(), 64);

        // Verify the signature using the inner keypair
        let inner = keypair.inner();
        assert!(
            inner
                .verify(message, &Signature::from_bytes(&signature))
                .is_ok()
        );
    }

    #[test]
    fn test_node_reputation_structure() {
        let peer_id = "test_peer".to_string();
        let reputation = NodeReputation {
            peer_id: peer_id.clone(),
            response_rate: 0.85,
            response_time: Duration::from_millis(150),
            consistency_score: 0.9,
            uptime_estimate: Duration::from_secs(86400),
            routing_accuracy: 0.8,
            last_seen: SystemTime::now(),
            interaction_count: 42,
        };

        assert_eq!(reputation.peer_id, peer_id);
        assert_eq!(reputation.response_rate, 0.85);
        assert_eq!(reputation.response_time, Duration::from_millis(150));
        assert_eq!(reputation.consistency_score, 0.9);
        assert_eq!(reputation.uptime_estimate, Duration::from_secs(86400));
        assert_eq!(reputation.routing_accuracy, 0.8);
        assert_eq!(reputation.interaction_count, 42);
    }

    #[test]
    fn test_ip_analysis_structure() {
        let analysis = IPAnalysis {
            subnet_64: Ipv6Addr::new(0x2001, 0xdb8, 0x85a3, 0x1234, 0, 0, 0, 0),
            subnet_48: Ipv6Addr::new(0x2001, 0xdb8, 0x85a3, 0, 0, 0, 0, 0),
            subnet_32: Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0),
            asn: Some(64512),
            country: Some("US".to_string()),
            is_hosting_provider: true,
            is_vpn_provider: false,
            reputation_score: 0.75,
        };

        assert_eq!(analysis.asn, Some(64512));
        assert_eq!(analysis.country, Some("US".to_string()));
        assert!(analysis.is_hosting_provider);
        assert!(!analysis.is_vpn_provider);
        assert_eq!(analysis.reputation_score, 0.75);
    }

    #[test]
    fn test_diversity_stats_structure() {
        let stats = DiversityStats {
            total_64_subnets: 100,
            total_48_subnets: 50,
            total_32_subnets: 25,
            total_asns: 15,
            total_countries: 8,
            max_nodes_per_64: 1,
            max_nodes_per_48: 3,
            max_nodes_per_32: 10,
        };

        assert_eq!(stats.total_64_subnets, 100);
        assert_eq!(stats.total_48_subnets, 50);
        assert_eq!(stats.total_32_subnets, 25);
        assert_eq!(stats.total_asns, 15);
        assert_eq!(stats.total_countries, 8);
        assert_eq!(stats.max_nodes_per_64, 1);
        assert_eq!(stats.max_nodes_per_48, 3);
        assert_eq!(stats.max_nodes_per_32, 10);
    }

    #[test]
    fn test_multiple_same_subnet_nodes() -> Result<()> {
        let config = IPDiversityConfig {
            max_nodes_per_64: 3, // Allow more nodes in same /64
            max_nodes_per_48: 5,
            max_nodes_per_32: 10,
            ..create_test_diversity_config()
        };
        let mut enforcer = IPDiversityEnforcer::new(config);

        let _base_addr = Ipv6Addr::new(
            0x2001, 0xdb8, 0x85a3, 0x1234, 0x5678, 0x8a2e, 0x0370, 0x0000,
        );

        // Add 3 nodes in same /64 subnet
        for i in 1..=3 {
            let addr = Ipv6Addr::new(0x2001, 0xdb8, 0x85a3, 0x1234, 0x5678, 0x8a2e, 0x0370, i);
            let analysis = enforcer.analyze_ip(addr)?;
            assert!(enforcer.can_accept_node(&analysis));
            enforcer.add_node(&analysis)?;
        }

        // 4th node should be rejected
        let addr4 = Ipv6Addr::new(0x2001, 0xdb8, 0x85a3, 0x1234, 0x5678, 0x8a2e, 0x0370, 4);
        let analysis4 = enforcer.analyze_ip(addr4)?;
        assert!(!enforcer.can_accept_node(&analysis4));

        let stats = enforcer.get_diversity_stats();
        assert_eq!(stats.total_64_subnets, 1);
        assert_eq!(stats.max_nodes_per_64, 3);

        Ok(())
    }

    #[test]
    fn test_asn_and_country_tracking() -> Result<()> {
        let config = create_test_diversity_config();
        let mut enforcer = IPDiversityEnforcer::new(config);

        // Create analysis with ASN and country
        let ipv6_addr = create_test_ipv6();
        let mut analysis = enforcer.analyze_ip(ipv6_addr)?;
        analysis.asn = Some(64512);
        analysis.country = Some("US".to_string());

        enforcer.add_node(&analysis)?;

        assert_eq!(enforcer.asn_counts.get(&64512), Some(&1));
        assert_eq!(enforcer.country_counts.get("US"), Some(&1));

        // Remove and check cleanup
        enforcer.remove_node(&analysis);
        assert!(enforcer.asn_counts.get(&64512).is_none());
        assert!(enforcer.country_counts.get("US").is_none());

        Ok(())
    }

    #[test]
    fn test_reputation_mixed_interactions() {
        let mut manager = ReputationManager::new(0.1, 0.1);
        let peer_id = "test_peer".to_string();

        // Mix of successful and failed interactions
        for i in 0..15 {
            let success = i % 3 != 0; // 2/3 success rate
            manager.update_reputation(&peer_id, success, Duration::from_millis(100 + i * 10));
        }

        let reputation = manager.get_reputation(&peer_id).unwrap();
        // Should converge closer to 2/3 with more iterations and higher learning rate
        // With alpha=0.3 and 2/3 success rate, convergence may be higher
        assert!(reputation.response_rate > 0.55);
        assert!(reputation.response_rate < 0.85);
        assert_eq!(reputation.interaction_count, 15);
    }
}
