//! IPv6-based DHT Node Identity System
//!
//! This module provides IPv6-based node identity for the DHT, integrating network-level
//! security with application-level S/Kademlia protections. It ensures that DHT node IDs
//! are cryptographically bound to actual IPv6 addresses, preventing various attack vectors.

use crate::dht::{Key, DHTNode};
use crate::security::{IPv6NodeID, IPDiversityEnforcer, IPDiversityConfig};
use crate::{PeerId, Result, P2PError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::Ipv6Addr;
use std::time::{Duration, SystemTime};
use tracing::{debug, info, warn};

/// IPv6-based DHT node identity that binds node ID to network location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IPv6DHTNode {
    /// Base DHT node information
    pub base_node: DHTNode,
    /// IPv6-based node identity
    pub ipv6_identity: IPv6NodeID,
    /// IP diversity analysis
    pub ip_analysis: crate::security::IPAnalysis,
    /// Security validation timestamp
    pub validated_at: SystemTime,
    /// Identity verification status
    pub is_verified: bool,
}

/// Configuration for IPv6-DHT integration
#[derive(Debug, Clone)]
pub struct IPv6DHTConfig {
    /// IPv6 diversity enforcement settings
    pub diversity_config: IPDiversityConfig,
    /// Enable IPv6 identity verification for all operations
    pub enable_ipv6_verification: bool,
    /// Enable IP diversity enforcement
    pub enable_ip_diversity: bool,
    /// Minimum node reputation for IPv6 operations
    pub min_ipv6_reputation: f64,
    /// IPv6 identity refresh interval
    pub identity_refresh_interval: Duration,
    /// Maximum age for cached IP analysis
    pub ip_analysis_cache_ttl: Duration,
    /// Enable automatic node banning for security violations
    pub enable_node_banning: bool,
    /// Ban duration for security violations
    pub security_ban_duration: Duration,
}

/// IPv6-based DHT identity manager
#[derive(Debug)]
pub struct IPv6DHTIdentityManager {
    /// Configuration
    pub config: IPv6DHTConfig,
    /// IP diversity enforcer
    pub ip_enforcer: IPDiversityEnforcer,
    /// Verified IPv6 nodes
    verified_nodes: HashMap<PeerId, IPv6DHTNode>,
    /// Node identity cache
    identity_cache: HashMap<PeerId, (IPv6NodeID, SystemTime)>,
    /// IP analysis cache
    ip_analysis_cache: HashMap<Ipv6Addr, (crate::security::IPAnalysis, SystemTime)>,
    /// Banned nodes for security violations
    banned_nodes: HashMap<PeerId, SystemTime>,
    /// Local IPv6 identity
    local_identity: Option<IPv6NodeID>,
}

/// IPv6 identity verification result
#[derive(Debug, Clone)]
pub struct IPv6VerificationResult {
    /// Verification success
    pub is_valid: bool,
    /// Verification confidence (0.0-1.0)
    pub confidence: f64,
    /// Error message if verification failed
    pub error_message: Option<String>,
    /// IP diversity check result
    pub ip_diversity_ok: bool,
    /// Identity freshness (age in seconds)
    pub identity_age_secs: u64,
}

/// Security event for IPv6-DHT integration
#[derive(Debug, Clone)]
pub enum IPv6SecurityEvent {
    /// Node joined with valid IPv6 identity
    NodeJoined {
        /// ID of the peer that joined
        peer_id: PeerId,
        /// IPv6 address of the peer
        ipv6_addr: Ipv6Addr,
        /// Confidence level of identity verification (0.0-1.0)
        verification_confidence: f64,
    },
    /// Node failed IPv6 verification
    VerificationFailed {
        /// ID of the peer that failed verification
        peer_id: PeerId,
        /// IPv6 address that failed verification
        ipv6_addr: Ipv6Addr,
        /// Reason for verification failure
        reason: String,
    },
    /// IP diversity violation detected
    DiversityViolation {
        /// ID of the peer causing violation
        peer_id: PeerId,
        /// IPv6 address involved in violation
        ipv6_addr: Ipv6Addr,
        /// Type of subnet causing the violation
        subnet_type: String,
    },
    /// Node banned for security violations
    NodeBanned {
        /// ID of the banned peer
        peer_id: PeerId,
        /// IPv6 address of the banned peer
        ipv6_addr: Ipv6Addr,
        /// Reason for banning
        reason: String,
        /// Duration of the ban
        ban_duration: Duration,
    },
    /// Suspicious activity detected
    SuspiciousActivity {
        /// ID of the suspicious peer
        peer_id: PeerId,
        /// IPv6 address of the suspicious peer
        ipv6_addr: Ipv6Addr,
        /// Type of suspicious activity detected
        activity_type: String,
    },
}

impl Default for IPv6DHTConfig {
    fn default() -> Self {
        Self {
            diversity_config: IPDiversityConfig::default(),
            enable_ipv6_verification: true,
            enable_ip_diversity: true,
            min_ipv6_reputation: 0.3,
            identity_refresh_interval: Duration::from_secs(3600), // 1 hour
            ip_analysis_cache_ttl: Duration::from_secs(1800), // 30 minutes
            enable_node_banning: true,
            security_ban_duration: Duration::from_secs(7200), // 2 hours
        }
    }
}

impl IPv6DHTIdentityManager {
    /// Create a new IPv6 DHT identity manager
    pub fn new(config: IPv6DHTConfig) -> Self {
        let ip_enforcer = IPDiversityEnforcer::new(config.diversity_config.clone());
        
        Self {
            config,
            ip_enforcer,
            verified_nodes: HashMap::new(),
            identity_cache: HashMap::new(),
            ip_analysis_cache: HashMap::new(),
            banned_nodes: HashMap::new(),
            local_identity: None,
        }
    }

    /// Set the local IPv6 identity
    pub fn set_local_identity(&mut self, identity: IPv6NodeID) -> Result<()> {
        // Verify the local identity
        match identity.verify() {
            Ok(true) => {
                self.local_identity = Some(identity);
                info!("Local IPv6 identity set and verified");
                Ok(())
            }
            Ok(false) => {
                Err(P2PError::Security("Local IPv6 identity verification failed".to_string()).into())
            }
            Err(e) => {
                Err(P2PError::Security(format!("Identity verification error: {}", e)).into())
            }
        }
    }

    /// Generate DHT key from IPv6 node identity
    pub fn generate_dht_key(ipv6_identity: &IPv6NodeID) -> Key {
        // Use the node_id from IPv6 identity as the DHT key
        // This ensures the DHT key is cryptographically bound to the IPv6 address
        Key::from_hash(
            ipv6_identity.node_id.as_slice()
                .try_into()
                .unwrap_or([0u8; 32])
        )
    }

    /// Convert a regular DHT node to IPv6-enhanced node
    pub async fn enhance_dht_node(&mut self, node: DHTNode, ipv6_identity: IPv6NodeID) -> Result<IPv6DHTNode> {
        // Verify IPv6 identity
        let verification_result = self.verify_ipv6_identity(&ipv6_identity).await?;
        
        if !verification_result.is_valid {
            return Err(P2PError::Security(format!(
                "IPv6 identity verification failed: {}",
                verification_result.error_message.unwrap_or_default()
            )).into());
        }

        // Analyze IP for diversity enforcement
        let ip_analysis = self.analyze_node_ip(ipv6_identity.ipv6_addr).await?;

        // Check IP diversity constraints
        if self.config.enable_ip_diversity && !self.ip_enforcer.can_accept_node(&ip_analysis) {
            return Err(P2PError::Security(
                "IP diversity constraints violated".to_string()
            ).into());
        }

        // Add to IP diversity tracking
        if self.config.enable_ip_diversity {
            self.ip_enforcer.add_node(&ip_analysis)
                .map_err(|e| P2PError::Security(format!("IP diversity error: {}", e)))?;
        }

        let enhanced_node = IPv6DHTNode {
            base_node: node,
            ipv6_identity,
            ip_analysis,
            validated_at: SystemTime::now(),
            is_verified: verification_result.is_valid,
        };

        // Cache the verified node
        self.verified_nodes.insert(enhanced_node.base_node.peer_id.clone(), enhanced_node.clone());

        info!("Enhanced DHT node with IPv6 identity: {}", enhanced_node.base_node.peer_id);
        Ok(enhanced_node)
    }

    /// Verify IPv6 node identity
    pub async fn verify_ipv6_identity(&mut self, identity: &IPv6NodeID) -> Result<IPv6VerificationResult> {
        // Check cache first
        if let Some((cached_identity, cached_at)) = self.identity_cache.get(&identity.ipv6_addr.to_string()) {
            if cached_at.elapsed().unwrap_or(Duration::MAX) < self.config.identity_refresh_interval {
                if cached_identity.node_id == identity.node_id {
                    return Ok(IPv6VerificationResult {
                        is_valid: true,
                        confidence: 0.9, // High confidence for cached valid identity
                        error_message: None,
                        ip_diversity_ok: true,
                        identity_age_secs: cached_at.elapsed().unwrap_or_default().as_secs(),
                    });
                }
            }
        }

        // Verify cryptographic signature
        let signature_valid = match identity.verify() {
            Ok(valid) => valid,
            Err(e) => {
                warn!("IPv6 identity signature verification failed: {}", e);
                return Ok(IPv6VerificationResult {
                    is_valid: false,
                    confidence: 0.0,
                    error_message: Some(format!("Signature verification failed: {}", e)),
                    ip_diversity_ok: false,
                    identity_age_secs: 0,
                });
            }
        };

        if !signature_valid {
            return Ok(IPv6VerificationResult {
                is_valid: false,
                confidence: 0.0,
                error_message: Some("Invalid cryptographic signature".to_string()),
                ip_diversity_ok: false,
                identity_age_secs: 0,
            });
        }

        // Check identity freshness
        let identity_age = identity.timestamp_secs;
        let now_secs = SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default().as_secs();
        let age_secs = now_secs.saturating_sub(identity_age);

        // Reject identities older than 24 hours
        if age_secs > 86400 {
            return Ok(IPv6VerificationResult {
                is_valid: false,
                confidence: 0.0,
                error_message: Some("Identity too old".to_string()),
                ip_diversity_ok: false,
                identity_age_secs: age_secs,
            });
        }

        // Analyze IP for diversity
        let ip_analysis = self.analyze_node_ip(identity.ipv6_addr).await?;
        let ip_diversity_ok = !self.config.enable_ip_diversity || 
                             self.ip_enforcer.can_accept_node(&ip_analysis);

        // Calculate confidence based on various factors
        let mut confidence = 1.0;
        
        // Reduce confidence for old identities
        if age_secs > 3600 { // Older than 1 hour
            confidence -= (age_secs as f64 - 3600.0) / 86400.0 * 0.3;
        }

        // Reduce confidence for hosting providers
        if ip_analysis.is_hosting_provider {
            confidence -= 0.2;
        }

        // Reduce confidence for VPN providers
        if ip_analysis.is_vpn_provider {
            confidence -= 0.3;
        }

        confidence = confidence.max(0.0).min(1.0);

        // Cache the identity
        self.identity_cache.insert(
            identity.ipv6_addr.to_string(),
            (identity.clone(), SystemTime::now())
        );

        Ok(IPv6VerificationResult {
            is_valid: signature_valid && ip_diversity_ok && confidence >= self.config.min_ipv6_reputation,
            confidence,
            error_message: None,
            ip_diversity_ok,
            identity_age_secs: age_secs,
        })
    }

    /// Analyze node IP for diversity enforcement
    async fn analyze_node_ip(&mut self, ipv6_addr: Ipv6Addr) -> Result<crate::security::IPAnalysis> {
        // Check cache first
        if let Some((cached_analysis, cached_at)) = self.ip_analysis_cache.get(&ipv6_addr) {
            if cached_at.elapsed().unwrap_or(Duration::MAX) < self.config.ip_analysis_cache_ttl {
                return Ok(cached_analysis.clone());
            }
        }

        // Perform IP analysis
        let analysis = self.ip_enforcer.analyze_ip(ipv6_addr)
            .map_err(|e| P2PError::Security(format!("IP analysis error: {}", e)))?;

        // Cache the analysis
        self.ip_analysis_cache.insert(ipv6_addr, (analysis.clone(), SystemTime::now()));

        Ok(analysis)
    }

    /// Validate node join with IPv6 security checks
    pub async fn validate_node_join(&mut self, node: &DHTNode, ipv6_identity: &IPv6NodeID) -> Result<IPv6SecurityEvent> {
        // Check if node is banned
        if let Some(ban_time) = self.banned_nodes.get(&node.peer_id) {
            if ban_time.elapsed().unwrap_or(Duration::MAX) < self.config.security_ban_duration {
                return Ok(IPv6SecurityEvent::NodeBanned {
                    peer_id: node.peer_id.clone(),
                    ipv6_addr: ipv6_identity.ipv6_addr,
                    reason: "Node still banned".to_string(),
                    ban_duration: self.config.security_ban_duration,
                });
            } else {
                // Remove expired ban
                self.banned_nodes.remove(&node.peer_id);
            }
        }

        // Verify IPv6 identity
        let verification_result = self.verify_ipv6_identity(ipv6_identity).await?;

        if !verification_result.is_valid {
            let event = IPv6SecurityEvent::VerificationFailed {
                peer_id: node.peer_id.clone(),
                ipv6_addr: ipv6_identity.ipv6_addr,
                reason: verification_result.error_message.unwrap_or("Unknown".to_string()),
            };

            // Ban node for repeated verification failures
            if self.config.enable_node_banning {
                self.banned_nodes.insert(node.peer_id.clone(), SystemTime::now());
            }

            return Ok(event);
        }

        // Check IP diversity
        let ip_analysis = self.analyze_node_ip(ipv6_identity.ipv6_addr).await?;
        
        if self.config.enable_ip_diversity && !self.ip_enforcer.can_accept_node(&ip_analysis) {
            return Ok(IPv6SecurityEvent::DiversityViolation {
                peer_id: node.peer_id.clone(),
                ipv6_addr: ipv6_identity.ipv6_addr,
                subnet_type: "IPv6 subnet".to_string(),
            });
        }

        // Node join is valid
        Ok(IPv6SecurityEvent::NodeJoined {
            peer_id: node.peer_id.clone(),
            ipv6_addr: ipv6_identity.ipv6_addr,
            verification_confidence: verification_result.confidence,
        })
    }

    /// Get verified IPv6 node by peer ID
    pub fn get_verified_node(&self, peer_id: &PeerId) -> Option<&IPv6DHTNode> {
        self.verified_nodes.get(peer_id)
    }

    /// Remove node from IPv6 tracking
    pub fn remove_node(&mut self, peer_id: &PeerId) {
        if let Some(ipv6_node) = self.verified_nodes.remove(peer_id) {
            // Remove from IP diversity tracking
            self.ip_enforcer.remove_node(&ipv6_node.ip_analysis);
            debug!("Removed IPv6 node from tracking: {}", peer_id);
        }
    }

    /// Check if node is banned
    pub fn is_node_banned(&self, peer_id: &PeerId) -> bool {
        if let Some(ban_time) = self.banned_nodes.get(peer_id) {
            ban_time.elapsed().unwrap_or(Duration::MAX) < self.config.security_ban_duration
        } else {
            false
        }
    }

    /// Ban a node for security violations
    pub fn ban_node(&mut self, peer_id: &PeerId, reason: &str) {
        self.banned_nodes.insert(peer_id.clone(), SystemTime::now());
        warn!("Banned node {} for: {}", peer_id, reason);
    }

    /// Get IPv6 diversity statistics
    pub fn get_ipv6_diversity_stats(&self) -> crate::security::DiversityStats {
        self.ip_enforcer.get_diversity_stats()
    }

    /// Cleanup expired entries
    pub fn cleanup_expired(&mut self) {
        let _now = SystemTime::now();

        // Remove expired identity cache entries
        self.identity_cache.retain(|_, (_, cached_at)| {
            cached_at.elapsed().unwrap_or(Duration::MAX) < self.config.identity_refresh_interval
        });

        // Remove expired IP analysis cache entries
        self.ip_analysis_cache.retain(|_, (_, cached_at)| {
            cached_at.elapsed().unwrap_or(Duration::MAX) < self.config.ip_analysis_cache_ttl
        });

        // Remove expired bans
        self.banned_nodes.retain(|_, ban_time| {
            ban_time.elapsed().unwrap_or(Duration::MAX) < self.config.security_ban_duration
        });

        // Remove old verified nodes
        self.verified_nodes.retain(|_, node| {
            node.validated_at.elapsed().unwrap_or(Duration::MAX) < Duration::from_secs(86400)
        });
    }

    /// Get local IPv6 identity
    pub fn get_local_identity(&self) -> Option<&IPv6NodeID> {
        self.local_identity.as_ref()
    }

    /// Update node reputation based on IPv6 behavior
    pub fn update_ipv6_reputation(&mut self, peer_id: &PeerId, positive_behavior: bool) {
        if let Some(ipv6_node) = self.verified_nodes.get_mut(peer_id) {
            // Update reputation score based on behavior
            if positive_behavior {
                ipv6_node.ip_analysis.reputation_score = 
                    (ipv6_node.ip_analysis.reputation_score + 0.1).min(1.0);
            } else {
                ipv6_node.ip_analysis.reputation_score = 
                    (ipv6_node.ip_analysis.reputation_score - 0.2).max(0.0);
                
                // Ban node if reputation drops too low
                if ipv6_node.ip_analysis.reputation_score < 0.1 && self.config.enable_node_banning {
                    self.ban_node(peer_id, "Low IPv6 reputation");
                }
            }
        }
    }
}

impl IPv6DHTNode {
    /// Create a new IPv6 DHT node
    pub fn new(base_node: DHTNode, ipv6_identity: IPv6NodeID, ip_analysis: crate::security::IPAnalysis) -> Self {
        Self {
            base_node,
            ipv6_identity,
            ip_analysis,
            validated_at: SystemTime::now(),
            is_verified: false,
        }
    }

    /// Get the DHT key derived from IPv6 identity
    pub fn get_dht_key(&self) -> Key {
        IPv6DHTIdentityManager::generate_dht_key(&self.ipv6_identity)
    }

    /// Check if identity needs refresh
    pub fn needs_identity_refresh(&self, refresh_interval: Duration) -> bool {
        self.validated_at.elapsed().unwrap_or(Duration::MAX) > refresh_interval
    }

    /// Get IPv6 subnet information
    pub fn get_subnet_info(&self) -> (Ipv6Addr, Ipv6Addr, Ipv6Addr) {
        (
            self.ipv6_identity.extract_subnet_64(),
            self.ipv6_identity.extract_subnet_48(),
            self.ipv6_identity.extract_subnet_32(),
        )
    }
}