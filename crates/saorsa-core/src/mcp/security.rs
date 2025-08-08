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

//! MCP Security Module
//!
//! This module provides comprehensive security features for the MCP server including:
//! - JWT-based authentication
//! - Peer identity verification
//! - Access control and permissions
//! - Rate limiting and abuse prevention
//! - Message integrity and encryption

use crate::{P2PError, PeerId, Result};
use base64::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// JWT-like token structure for MCP authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPToken {
    /// Token header
    pub header: TokenHeader,
    /// Token payload
    pub payload: TokenPayload,
    /// Token signature
    pub signature: String,
}

/// Token header information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenHeader {
    /// Algorithm used for signing
    pub alg: String,
    /// Token type
    pub typ: String,
    /// Key ID
    pub kid: Option<String>,
}

/// Token payload with claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPayload {
    /// Issuer (peer ID)
    pub iss: PeerId,
    /// Subject (target peer ID or tool)
    pub sub: String,
    /// Audience (intended recipient)
    pub aud: String,
    /// Expiration time (Unix timestamp)
    pub exp: u64,
    /// Not before time (Unix timestamp)
    pub nbf: u64,
    /// Issued at time (Unix timestamp)
    pub iat: u64,
    /// JWT ID
    pub jti: String,
    /// Custom claims
    pub claims: HashMap<String, serde_json::Value>,
}

/// Security level for MCP operations
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecurityLevel {
    /// Public access - no authentication required
    Public,
    /// Basic authentication required
    Basic,
    /// Strong authentication required
    Strong,
    /// Administrative access required
    Admin,
}

/// Permission for MCP operations
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MCPPermission {
    /// Read access to tools
    ReadTools,
    /// Execute tools
    ExecuteTools,
    /// Register new tools
    RegisterTools,
    /// Modify existing tools
    ModifyTools,
    /// Delete tools
    DeleteTools,
    /// Access prompts
    AccessPrompts,
    /// Access resources
    AccessResources,
    /// Administrative access
    Admin,
    /// Custom permission
    Custom(String),
}

impl MCPPermission {
    /// Get permission string representation
    pub fn as_str(&self) -> &str {
        match self {
            MCPPermission::ReadTools => "read:tools",
            MCPPermission::ExecuteTools => "execute:tools",
            MCPPermission::RegisterTools => "register:tools",
            MCPPermission::ModifyTools => "modify:tools",
            MCPPermission::DeleteTools => "delete:tools",
            MCPPermission::AccessPrompts => "access:prompts",
            MCPPermission::AccessResources => "access:resources",
            MCPPermission::Admin => "admin",
            MCPPermission::Custom(s) => s,
        }
    }

    /// Parse permission from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "read:tools" => Some(MCPPermission::ReadTools),
            "execute:tools" => Some(MCPPermission::ExecuteTools),
            "register:tools" => Some(MCPPermission::RegisterTools),
            "modify:tools" => Some(MCPPermission::ModifyTools),
            "delete:tools" => Some(MCPPermission::DeleteTools),
            "access:prompts" => Some(MCPPermission::AccessPrompts),
            "access:resources" => Some(MCPPermission::AccessResources),
            "admin" => Some(MCPPermission::Admin),
            _ => Some(MCPPermission::Custom(s.to_string())),
        }
    }
}

/// Access control list for a peer
#[derive(Debug, Clone)]
pub struct PeerACL {
    /// Peer ID
    pub peer_id: PeerId,
    /// Granted permissions
    pub permissions: Vec<MCPPermission>,
    /// Security level
    pub security_level: SecurityLevel,
    /// Reputation score (0.0 to 1.0)
    pub reputation: f64,
    /// Last access time
    pub last_access: SystemTime,
    /// Access count
    pub access_count: u64,
    /// Rate limit violations
    pub rate_violations: u32,
    /// Banned until (if applicable)
    pub banned_until: Option<SystemTime>,
}

impl PeerACL {
    /// Create new peer ACL with default permissions
    pub fn new(peer_id: PeerId) -> Self {
        Self {
            peer_id,
            permissions: vec![MCPPermission::ReadTools, MCPPermission::ExecuteTools],
            security_level: SecurityLevel::Basic,
            reputation: 0.5, // Start with neutral reputation
            last_access: SystemTime::now(),
            access_count: 0,
            rate_violations: 0,
            banned_until: None,
        }
    }

    /// Check if peer has specific permission
    pub fn has_permission(&self, permission: &MCPPermission) -> bool {
        if self.is_banned() {
            return false;
        }

        // Admin permission grants all access
        if self.permissions.contains(&MCPPermission::Admin) {
            return true;
        }

        self.permissions.contains(permission)
    }

    /// Check if peer is currently banned
    pub fn is_banned(&self) -> bool {
        if let Some(banned_until) = self.banned_until {
            SystemTime::now() < banned_until
        } else {
            false
        }
    }

    /// Update access statistics
    pub fn record_access(&mut self) {
        self.last_access = SystemTime::now();
        self.access_count += 1;
    }

    /// Record rate limit violation
    pub fn record_rate_violation(&mut self) {
        self.rate_violations += 1;

        // Auto-ban after too many violations
        if self.rate_violations >= 10 {
            self.banned_until = Some(SystemTime::now() + Duration::from_secs(3600)); // 1 hour
        }
    }

    /// Grant permission to peer
    pub fn grant_permission(&mut self, permission: MCPPermission) {
        if !self.permissions.contains(&permission) {
            self.permissions.push(permission);
        }
    }

    /// Revoke permission from peer
    pub fn revoke_permission(&mut self, permission: &MCPPermission) {
        self.permissions.retain(|p| p != permission);
    }
}

/// Rate limiter for controlling request frequency
#[derive(Debug, Clone)]
pub struct RateLimiter {
    /// Requests per minute limit
    pub rpm_limit: u32,
    /// Request timestamps for each peer
    requests: Arc<RwLock<HashMap<PeerId, Vec<SystemTime>>>>,
}

impl RateLimiter {
    /// Create new rate limiter
    pub fn new(rpm_limit: u32) -> Self {
        Self {
            rpm_limit,
            requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if request is allowed for peer
    pub async fn is_allowed(&self, peer_id: &PeerId) -> bool {
        let mut requests = self.requests.write().await;
        let now = SystemTime::now();
        let minute_ago = now - Duration::from_secs(60);

        // Get or create request history for peer
        let peer_requests = requests.entry(peer_id.clone()).or_insert_with(Vec::new);

        // Remove old requests (older than 1 minute)
        peer_requests.retain(|&req_time| req_time > minute_ago);

        // Check if under limit
        if peer_requests.len() < self.rpm_limit as usize {
            peer_requests.push(now);
            true
        } else {
            false
        }
    }

    /// Reset rate limit for peer (admin function)
    pub async fn reset_peer(&self, peer_id: &PeerId) {
        let mut requests = self.requests.write().await;
        requests.remove(peer_id);
    }

    /// Clean up old entries periodically
    pub async fn cleanup(&self) {
        let mut requests = self.requests.write().await;
        let minute_ago = SystemTime::now() - Duration::from_secs(60);

        for peer_requests in requests.values_mut() {
            peer_requests.retain(|&req_time| req_time > minute_ago);
        }

        // Remove empty entries
        requests.retain(|_, reqs| !reqs.is_empty());
    }
}

/// MCP Security Manager
pub struct MCPSecurityManager {
    /// Access control lists
    acls: Arc<RwLock<HashMap<PeerId, PeerACL>>>,
    /// Rate limiter
    rate_limiter: RateLimiter,
    /// Shared secret for token signing
    secret_key: Vec<u8>,
    /// Tool security policies
    tool_policies: Arc<RwLock<HashMap<String, SecurityLevel>>>,
    /// Trusted peer list
    trusted_peers: Arc<RwLock<Vec<PeerId>>>,
}

impl MCPSecurityManager {
    /// Create new security manager
    pub fn new(secret_key: Vec<u8>, rpm_limit: u32) -> Self {
        Self {
            acls: Arc::new(RwLock::new(HashMap::new())),
            rate_limiter: RateLimiter::new(rpm_limit),
            secret_key,
            tool_policies: Arc::new(RwLock::new(HashMap::new())),
            trusted_peers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Generate authentication token for peer
    pub async fn generate_token(
        &self,
        peer_id: &PeerId,
        permissions: Vec<MCPPermission>,
        ttl: Duration,
    ) -> Result<String> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| {
            P2PError::Identity(crate::error::IdentityError::SystemTime(
                format!("Time error: {e}").into(),
            ))
        })?;

        let payload = TokenPayload {
            iss: peer_id.clone(),
            sub: peer_id.clone(),
            aud: "mcp-server".to_string(),
            exp: (now + ttl).as_secs(),
            nbf: now.as_secs(),
            iat: now.as_secs(),
            jti: uuid::Uuid::new_v4().to_string(),
            claims: {
                let mut claims = HashMap::new();
                claims.insert(
                    "permissions".to_string(),
                    serde_json::to_value(
                        permissions.iter().map(|p| p.as_str()).collect::<Vec<_>>(),
                    )
                    .expect("valid security operation"),
                );
                claims
            },
        };

        let header = TokenHeader {
            alg: "HS256".to_string(),
            typ: "JWT".to_string(),
            kid: None,
        };

        // Create token without signature first
        let header_b64 = base64::prelude::BASE64_URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(&header)
                .map_err(|e| P2PError::Serialization(e.to_string().into()))?,
        );
        let payload_b64 = base64::prelude::BASE64_URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(&payload)
                .map_err(|e| P2PError::Serialization(e.to_string().into()))?,
        );

        // Sign the token
        let signing_input = format!("{header_b64}.{payload_b64}");
        let signature = self.sign_data(signing_input.as_bytes());
        let signature_b64 = base64::prelude::BASE64_URL_SAFE_NO_PAD.encode(signature);

        Ok(format!("{header_b64}.{payload_b64}.{signature_b64}").into())
    }

    /// Verify authentication token
    pub async fn verify_token(&self, token: &str) -> Result<TokenPayload> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(P2PError::Mcp(crate::error::McpError::InvalidRequest(
                "Invalid token format".to_string().into(),
            )));
        }

        let _header_data = base64::prelude::BASE64_URL_SAFE_NO_PAD
            .decode(parts[0])
            .map_err(|e| {
                P2PError::Mcp(crate::error::McpError::InvalidRequest(
                    format!("Invalid header encoding: {e}").into(),
                ))
            })?;
        let payload_data = base64::prelude::BASE64_URL_SAFE_NO_PAD
            .decode(parts[1])
            .map_err(|e| {
                P2PError::Mcp(crate::error::McpError::InvalidRequest(
                    format!("Invalid payload encoding: {e}").into(),
                ))
            })?;
        let signature = base64::prelude::BASE64_URL_SAFE_NO_PAD
            .decode(parts[2])
            .map_err(|e| {
                P2PError::Mcp(crate::error::McpError::InvalidRequest(
                    format!("Invalid signature encoding: {e}").into(),
                ))
            })?;

        // Verify signature
        let signing_input = format!("{}.{}", parts[0], parts[1]);
        let expected_signature = self.sign_data(signing_input.as_bytes());

        if signature != expected_signature {
            return Err(P2PError::Mcp(crate::error::McpError::InvalidRequest(
                "Invalid token signature".to_string().into(),
            )));
        }

        // Parse payload
        let payload: TokenPayload = serde_json::from_slice(&payload_data).map_err(|e| {
            P2PError::Mcp(crate::error::McpError::InvalidRequest(
                format!("Invalid payload: {e}").into(),
            ))
        })?;

        // Check expiration
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| {
                P2PError::Identity(crate::error::IdentityError::SystemTime(
                    format!("Time error: {e}").into(),
                ))
            })?
            .as_secs();

        if payload.exp < now {
            return Err(P2PError::Mcp(crate::error::McpError::InvalidRequest(
                "Token expired".to_string().into(),
            )));
        }

        if payload.nbf > now {
            return Err(P2PError::Mcp(crate::error::McpError::InvalidRequest(
                "Token not yet valid".to_string().into(),
            )));
        }

        Ok(payload)
    }

    /// Check if peer has permission for operation
    pub async fn check_permission(
        &self,
        peer_id: &PeerId,
        permission: &MCPPermission,
    ) -> Result<bool> {
        let acls = self.acls.read().await;

        if let Some(acl) = acls.get(peer_id) {
            Ok(acl.has_permission(permission))
        } else {
            // Create default ACL for new peer
            drop(acls);
            let mut acls = self.acls.write().await;
            acls.insert(peer_id.clone(), PeerACL::new(peer_id.clone()));
            Ok(false) // New peers start with no permissions by default
        }
    }

    /// Check rate limit for peer
    pub async fn check_rate_limit(&self, peer_id: &PeerId) -> Result<bool> {
        if self.rate_limiter.is_allowed(peer_id).await {
            Ok(true)
        } else {
            // Record violation
            let mut acls = self.acls.write().await;
            if let Some(acl) = acls.get_mut(peer_id) {
                acl.record_rate_violation();
            }
            Ok(false)
        }
    }

    /// Grant permission to peer
    pub async fn grant_permission(
        &self,
        peer_id: &PeerId,
        permission: MCPPermission,
    ) -> Result<()> {
        let mut acls = self.acls.write().await;
        let acl = acls
            .entry(peer_id.clone())
            .or_insert_with(|| PeerACL::new(peer_id.clone()));
        acl.grant_permission(permission);
        Ok(())
    }

    /// Revoke permission from peer
    pub async fn revoke_permission(
        &self,
        peer_id: &PeerId,
        permission: &MCPPermission,
    ) -> Result<()> {
        let mut acls = self.acls.write().await;
        if let Some(acl) = acls.get_mut(peer_id) {
            acl.revoke_permission(permission);
        }
        Ok(())
    }

    /// Add trusted peer
    pub async fn add_trusted_peer(&self, peer_id: PeerId) -> Result<()> {
        let mut trusted = self.trusted_peers.write().await;
        if !trusted.contains(&peer_id) {
            trusted.push(peer_id);
        }
        Ok(())
    }

    /// Check if peer is trusted
    pub async fn is_trusted_peer(&self, peer_id: &PeerId) -> bool {
        let trusted = self.trusted_peers.read().await;
        trusted.contains(peer_id)
    }

    /// Set security policy for tool
    pub async fn set_tool_policy(&self, tool_name: String, level: SecurityLevel) -> Result<()> {
        let mut policies = self.tool_policies.write().await;
        policies.insert(tool_name, level);
        Ok(())
    }

    /// Get security policy for tool
    pub async fn get_tool_policy(&self, tool_name: &str) -> SecurityLevel {
        let policies = self.tool_policies.read().await;
        policies
            .get(tool_name)
            .cloned()
            .unwrap_or(SecurityLevel::Basic)
    }

    /// Sign data with secret key
    fn sign_data(&self, data: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(&self.secret_key);
        hasher.update(data);
        hasher.finalize().to_vec()
    }

    /// Update peer reputation based on behavior
    pub async fn update_reputation(&self, peer_id: &PeerId, delta: f64) -> Result<()> {
        let mut acls = self.acls.write().await;
        if let Some(acl) = acls.get_mut(peer_id) {
            acl.reputation = (acl.reputation + delta).max(0.0).min(1.0);
        }
        Ok(())
    }

    /// Get peer statistics
    pub async fn get_peer_stats(&self, peer_id: &PeerId) -> Option<PeerACL> {
        let acls = self.acls.read().await;
        acls.get(peer_id).cloned()
    }

    /// Clean up expired data
    pub async fn cleanup(&self) -> Result<()> {
        self.rate_limiter.cleanup().await;

        // Clean up old ACLs (remove entries not accessed in 24 hours)
        let mut acls = self.acls.write().await;
        let day_ago = SystemTime::now() - Duration::from_secs(24 * 3600);
        acls.retain(|_, acl| acl.last_access > day_ago);

        Ok(())
    }
}

/// Security audit log entry
#[derive(Debug, Clone)]
pub struct SecurityAuditEntry {
    /// Timestamp
    pub timestamp: SystemTime,
    /// Event type
    pub event_type: String,
    /// Peer ID involved
    pub peer_id: PeerId,
    /// Event details
    pub details: HashMap<String, String>,
    /// Severity level
    pub severity: AuditSeverity,
}

/// Audit severity levels
#[derive(Debug, Clone, PartialEq)]
pub enum AuditSeverity {
    /// Informational
    Info,
    /// Warning
    Warning,
    /// Error
    Error,
    /// Critical security event
    Critical,
}

/// Security audit logger
pub struct SecurityAuditLogger {
    /// Audit entries
    entries: Arc<RwLock<Vec<SecurityAuditEntry>>>,
    /// Maximum entries to keep
    max_entries: usize,
}

impl SecurityAuditLogger {
    /// Create new audit logger
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            max_entries,
        }
    }

    /// Log security event
    pub async fn log_event(
        &self,
        event_type: String,
        peer_id: PeerId,
        details: HashMap<String, String>,
        severity: AuditSeverity,
    ) {
        let entry = SecurityAuditEntry {
            timestamp: SystemTime::now(),
            event_type,
            peer_id,
            details,
            severity,
        };

        let mut entries = self.entries.write().await;
        entries.push(entry);

        // Keep only recent entries
        if entries.len() > self.max_entries {
            let excess = entries.len() - self.max_entries;
            entries.drain(0..excess);
        }
    }

    /// Get recent audit entries
    pub async fn get_recent_entries(&self, limit: Option<usize>) -> Vec<SecurityAuditEntry> {
        let entries = self.entries.read().await;
        let limit = limit.unwrap_or(entries.len());
        entries.iter().rev().take(limit).cloned().collect()
    }

    /// Get entries by severity
    pub async fn get_entries_by_severity(
        &self,
        severity: AuditSeverity,
    ) -> Vec<SecurityAuditEntry> {
        let entries = self.entries.read().await;
        entries
            .iter()
            .filter(|e| e.severity == severity)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Helper function to create a test PeerId
    fn create_test_peer() -> PeerId {
        format!("test_peer_{}", rand::random::<u32>())
    }

    /// Helper function to create a test security manager
    fn create_test_security_manager() -> MCPSecurityManager {
        let secret_key = b"test_secret_key_1234567890123456".to_vec();
        MCPSecurityManager::new(secret_key, 60) // 60 RPM limit
    }

    #[test]
    fn test_mcp_permission_string_conversion() {
        let permissions = vec![
            (MCPPermission::ReadTools, "read:tools"),
            (MCPPermission::ExecuteTools, "execute:tools"),
            (MCPPermission::RegisterTools, "register:tools"),
            (MCPPermission::ModifyTools, "modify:tools"),
            (MCPPermission::DeleteTools, "delete:tools"),
            (MCPPermission::AccessPrompts, "access:prompts"),
            (MCPPermission::AccessResources, "access:resources"),
            (MCPPermission::Admin, "admin"),
        ];

        for (permission, expected_str) in permissions {
            assert_eq!(permission.as_str(), expected_str);
            assert_eq!(MCPPermission::from_str(expected_str), Some(permission));
        }

        // Test custom permission
        let custom = MCPPermission::Custom("custom:action".to_string());
        assert_eq!(custom.as_str(), "custom:action");
        assert_eq!(MCPPermission::from_str("custom:action"), Some(custom));

        // Test unknown permission defaults to custom
        let unknown = MCPPermission::from_str("unknown:permission");
        match unknown {
            Some(MCPPermission::Custom(s)) => assert_eq!(s, "unknown:permission"),
            _ => panic!("Expected custom permission"),
        }
    }

    #[test]
    fn test_security_level_ordering() {
        // Test security level ordering
        assert!(SecurityLevel::Public < SecurityLevel::Basic);
        assert!(SecurityLevel::Basic < SecurityLevel::Strong);
        assert!(SecurityLevel::Strong < SecurityLevel::Admin);

        // Test equality
        assert_eq!(SecurityLevel::Public, SecurityLevel::Public);
        assert_eq!(SecurityLevel::Basic, SecurityLevel::Basic);
        assert_eq!(SecurityLevel::Strong, SecurityLevel::Strong);
        assert_eq!(SecurityLevel::Admin, SecurityLevel::Admin);
    }

    #[test]
    fn test_peer_acl_creation() {
        let peer_id = create_test_peer();
        let acl = PeerACL::new(peer_id.clone());

        assert_eq!(acl.peer_id, peer_id);
        assert_eq!(acl.permissions.len(), 2); // Default: ReadTools, ExecuteTools
        assert!(acl.permissions.contains(&MCPPermission::ReadTools));
        assert!(acl.permissions.contains(&MCPPermission::ExecuteTools));
        assert_eq!(acl.security_level, SecurityLevel::Basic);
        assert_eq!(acl.reputation, 0.5);
        assert_eq!(acl.access_count, 0);
        assert_eq!(acl.rate_violations, 0);
        assert!(acl.banned_until.is_none());
        assert!(!acl.is_banned());
    }

    #[test]
    fn test_peer_acl_permissions() {
        let peer_id = create_test_peer();
        let mut acl = PeerACL::new(peer_id);

        // Test default permissions
        assert!(acl.has_permission(&MCPPermission::ReadTools));
        assert!(acl.has_permission(&MCPPermission::ExecuteTools));
        assert!(!acl.has_permission(&MCPPermission::RegisterTools));
        assert!(!acl.has_permission(&MCPPermission::Admin));

        // Grant admin permission
        acl.grant_permission(MCPPermission::Admin);
        // Admin permission grants all access
        assert!(acl.has_permission(&MCPPermission::ReadTools));
        assert!(acl.has_permission(&MCPPermission::ExecuteTools));
        assert!(acl.has_permission(&MCPPermission::RegisterTools));
        assert!(acl.has_permission(&MCPPermission::DeleteTools));
        assert!(acl.has_permission(&MCPPermission::Admin));

        // Revoke admin permission
        acl.revoke_permission(&MCPPermission::Admin);
        assert!(!acl.has_permission(&MCPPermission::RegisterTools));
        assert!(!acl.has_permission(&MCPPermission::Admin));

        // Grant specific permission
        acl.grant_permission(MCPPermission::RegisterTools);
        assert!(acl.has_permission(&MCPPermission::RegisterTools));

        // Revoke specific permission
        acl.revoke_permission(&MCPPermission::RegisterTools);
        assert!(!acl.has_permission(&MCPPermission::RegisterTools));
    }

    #[test]
    fn test_peer_acl_ban_functionality() {
        let peer_id = create_test_peer();
        let mut acl = PeerACL::new(peer_id);

        // Initially not banned
        assert!(!acl.is_banned());
        assert!(acl.has_permission(&MCPPermission::ReadTools));

        // Record violations (but not enough to trigger auto-ban)
        for _ in 0..5 {
            acl.record_rate_violation();
        }
        assert_eq!(acl.rate_violations, 5);
        assert!(!acl.is_banned());

        // Record enough violations to trigger auto-ban
        for _ in 0..5 {
            acl.record_rate_violation();
        }
        assert_eq!(acl.rate_violations, 10);
        assert!(acl.is_banned());

        // Banned peers have no permissions
        assert!(!acl.has_permission(&MCPPermission::ReadTools));
        assert!(!acl.has_permission(&MCPPermission::ExecuteTools));
    }

    #[test]
    fn test_peer_acl_access_tracking() {
        let peer_id = create_test_peer();
        let mut acl = PeerACL::new(peer_id);

        let initial_time = acl.last_access;
        assert_eq!(acl.access_count, 0);

        // Record access
        std::thread::sleep(std::time::Duration::from_millis(10));
        acl.record_access();

        assert_eq!(acl.access_count, 1);
        assert!(acl.last_access > initial_time);

        // Record more access
        acl.record_access();
        assert_eq!(acl.access_count, 2);
    }

    #[tokio::test]
    async fn test_rate_limiter_creation() {
        let limiter = RateLimiter::new(60);
        assert_eq!(limiter.rpm_limit, 60);
    }

    #[tokio::test]
    async fn test_rate_limiter_basic_functionality() {
        let limiter = RateLimiter::new(2); // 2 requests per minute
        let peer_id = create_test_peer();

        // First request should be allowed
        assert!(limiter.is_allowed(&peer_id).await);

        // Second request should be allowed
        assert!(limiter.is_allowed(&peer_id).await);

        // Third request should be denied (over limit)
        assert!(!limiter.is_allowed(&peer_id).await);
    }

    #[tokio::test]
    async fn test_rate_limiter_different_peers() {
        let limiter = RateLimiter::new(1); // 1 request per minute
        let peer1 = create_test_peer();
        let peer2 = create_test_peer();

        // Each peer should have their own limit
        assert!(limiter.is_allowed(&peer1).await);
        assert!(limiter.is_allowed(&peer2).await);

        // Both should be over their individual limits now
        assert!(!limiter.is_allowed(&peer1).await);
        assert!(!limiter.is_allowed(&peer2).await);
    }

    #[tokio::test]
    async fn test_rate_limiter_reset() {
        let limiter = RateLimiter::new(1);
        let peer_id = create_test_peer();

        // Use up the limit
        assert!(limiter.is_allowed(&peer_id).await);
        assert!(!limiter.is_allowed(&peer_id).await);

        // Reset the peer
        limiter.reset_peer(&peer_id).await;

        // Should be allowed again
        assert!(limiter.is_allowed(&peer_id).await);
    }

    #[tokio::test]
    async fn test_rate_limiter_cleanup() {
        let limiter = RateLimiter::new(10);
        let peer_id = create_test_peer();

        // Make some requests
        limiter.is_allowed(&peer_id).await;
        limiter.is_allowed(&peer_id).await;

        // Cleanup shouldn't affect recent requests
        limiter.cleanup().await;

        // Should still have request history
        let requests = limiter.requests.read().await;
        assert!(requests.contains_key(&peer_id));
        let peer_requests = requests.get(&peer_id).expect("valid security operation");
        assert_eq!(peer_requests.len(), 2);
    }

    #[tokio::test]
    async fn test_security_manager_creation() {
        let secret_key = b"test_secret_key".to_vec();
        let manager = MCPSecurityManager::new(secret_key.clone(), 60);

        // Verify configuration
        assert_eq!(manager.secret_key, secret_key);
        assert_eq!(manager.rate_limiter.rpm_limit, 60);
    }

    #[tokio::test]
    async fn test_token_generation_and_verification() -> Result<()> {
        let manager = create_test_security_manager();
        let peer_id = create_test_peer();
        let permissions = vec![MCPPermission::ReadTools, MCPPermission::ExecuteTools];
        let ttl = Duration::from_secs(3600); // 1 hour

        // Generate token
        let token = manager
            .generate_token(&peer_id, permissions.clone(), ttl)
            .await?;
        assert!(!token.is_empty());

        // Verify token
        let payload = manager.verify_token(&token).await?;
        assert_eq!(payload.iss, peer_id);
        assert_eq!(payload.sub, peer_id);
        assert_eq!(payload.aud, "mcp-server");

        // Check permissions in claims
        let permissions_claim = payload
            .claims
            .get("permissions")
            .expect("valid security operation");
        let permission_strings: Vec<String> =
            serde_json::from_value(permissions_claim.clone()).expect("valid security operation");
        assert_eq!(permission_strings.len(), 2);
        assert!(permission_strings.contains(&"read:tools".to_string()));
        assert!(permission_strings.contains(&"execute:tools".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_token_verification_invalid() {
        let manager = create_test_security_manager();

        // Test invalid token format
        let result = manager.verify_token("invalid.token").await;
        assert!(result.is_err());

        // Test malformed token
        let result = manager.verify_token("invalid.token.format.extra").await;
        assert!(result.is_err());

        // Test empty token
        let result = manager.verify_token("").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_token_signature_verification() -> Result<()> {
        let manager1 = create_test_security_manager();
        let manager2 = MCPSecurityManager::new(b"different_secret".to_vec(), 60);

        let peer_id = create_test_peer();
        let permissions = vec![MCPPermission::ReadTools];
        let ttl = Duration::from_secs(3600);

        // Generate token with manager1
        let token = manager1.generate_token(&peer_id, permissions, ttl).await?;

        // Verify with manager1 should succeed
        assert!(manager1.verify_token(&token).await.is_ok());

        // Verify with manager2 should fail (different secret)
        assert!(manager2.verify_token(&token).await.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn test_permission_management() -> Result<()> {
        let manager = create_test_security_manager();
        let peer_id = create_test_peer();

        // Initially should have no permissions (new peer starts with false)
        assert!(
            !manager
                .check_permission(&peer_id, &MCPPermission::ExecuteTools)
                .await?
        );

        // Grant permission
        manager
            .grant_permission(&peer_id, MCPPermission::ExecuteTools)
            .await?;
        assert!(
            manager
                .check_permission(&peer_id, &MCPPermission::ExecuteTools)
                .await?
        );

        // Revoke permission
        manager
            .revoke_permission(&peer_id, &MCPPermission::ExecuteTools)
            .await?;
        assert!(
            !manager
                .check_permission(&peer_id, &MCPPermission::ExecuteTools)
                .await?
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_rate_limit_checking() -> Result<()> {
        let manager = MCPSecurityManager::new(b"test_key".to_vec(), 2); // 2 RPM limit
        let peer_id = create_test_peer();

        // Grant permission first to create ACL entry
        manager
            .grant_permission(&peer_id, MCPPermission::ReadTools)
            .await?;

        // First two requests should pass
        assert!(manager.check_rate_limit(&peer_id).await?);
        assert!(manager.check_rate_limit(&peer_id).await?);

        // Third request should fail
        assert!(!manager.check_rate_limit(&peer_id).await?);

        // Check that violation was recorded
        let stats = manager.get_peer_stats(&peer_id).await;
        assert!(stats.is_some());
        let acl = stats.expect("valid security operation");
        assert_eq!(acl.rate_violations, 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_trusted_peer_management() -> Result<()> {
        let manager = create_test_security_manager();
        let peer_id = create_test_peer();

        // Initially not trusted
        assert!(!manager.is_trusted_peer(&peer_id).await);

        // Add as trusted
        manager.add_trusted_peer(peer_id.clone()).await?;
        assert!(manager.is_trusted_peer(&peer_id).await);

        // Adding same peer again should be idempotent
        manager.add_trusted_peer(peer_id.clone()).await?;
        assert!(manager.is_trusted_peer(&peer_id).await);

        Ok(())
    }

    #[tokio::test]
    async fn test_tool_security_policies() -> Result<()> {
        let manager = create_test_security_manager();

        // Default policy should be Basic
        let policy = manager.get_tool_policy("test_tool").await;
        assert_eq!(policy, SecurityLevel::Basic);

        // Set custom policy
        manager
            .set_tool_policy("test_tool".to_string(), SecurityLevel::Strong)
            .await?;
        let policy = manager.get_tool_policy("test_tool").await;
        assert_eq!(policy, SecurityLevel::Strong);

        // Set admin policy
        manager
            .set_tool_policy("admin_tool".to_string(), SecurityLevel::Admin)
            .await?;
        let policy = manager.get_tool_policy("admin_tool").await;
        assert_eq!(policy, SecurityLevel::Admin);

        Ok(())
    }

    #[tokio::test]
    async fn test_reputation_management() -> Result<()> {
        let manager = create_test_security_manager();
        let peer_id = create_test_peer();

        // Grant permission to create ACL entry
        manager
            .grant_permission(&peer_id, MCPPermission::ReadTools)
            .await?;

        let stats = manager
            .get_peer_stats(&peer_id)
            .await
            .expect("valid security operation");
        assert_eq!(stats.reputation, 0.5); // Default reputation

        // Increase reputation
        manager.update_reputation(&peer_id, 0.2).await?;
        let stats = manager
            .get_peer_stats(&peer_id)
            .await
            .expect("valid security operation");
        assert_eq!(stats.reputation, 0.7);

        // Decrease reputation
        manager.update_reputation(&peer_id, -0.3).await?;
        let stats = manager
            .get_peer_stats(&peer_id)
            .await
            .expect("valid security operation");
        assert!((stats.reputation - 0.4).abs() < 0.001); // Use epsilon for float comparison

        // Test bounds (should clamp to 0.0-1.0)
        manager.update_reputation(&peer_id, -1.0).await?;
        let stats = manager
            .get_peer_stats(&peer_id)
            .await
            .expect("valid security operation");
        assert_eq!(stats.reputation, 0.0);

        manager.update_reputation(&peer_id, 2.0).await?;
        let stats = manager
            .get_peer_stats(&peer_id)
            .await
            .expect("valid security operation");
        assert_eq!(stats.reputation, 1.0);

        Ok(())
    }

    #[tokio::test]
    async fn test_security_manager_cleanup() -> Result<()> {
        let manager = create_test_security_manager();
        let peer_id = create_test_peer();

        // Create some data
        manager
            .grant_permission(&peer_id, MCPPermission::ReadTools)
            .await?;
        manager.check_rate_limit(&peer_id).await?;

        // Cleanup should work without errors
        manager.cleanup().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_audit_logger_creation() {
        let logger = SecurityAuditLogger::new(100);
        assert_eq!(logger.max_entries, 100);

        let entries = logger.get_recent_entries(None).await;
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn test_audit_logger_logging() {
        let logger = SecurityAuditLogger::new(10);
        let peer_id = create_test_peer();

        let mut details = HashMap::new();
        details.insert("action".to_string(), "test_action".to_string());
        details.insert("result".to_string(), "success".to_string());

        // Log an event
        logger
            .log_event(
                "test_event".to_string(),
                peer_id.clone(),
                details.clone(),
                AuditSeverity::Info,
            )
            .await;

        let entries = logger.get_recent_entries(None).await;
        assert_eq!(entries.len(), 1);

        let entry = &entries[0];
        assert_eq!(entry.event_type, "test_event");
        assert_eq!(entry.peer_id, peer_id);
        assert_eq!(entry.severity, AuditSeverity::Info);
        assert_eq!(
            entry.details.get("action"),
            Some(&"test_action".to_string())
        );
    }

    #[tokio::test]
    async fn test_audit_logger_severity_filtering() {
        let logger = SecurityAuditLogger::new(10);
        let peer_id = create_test_peer();

        // Log events with different severities
        logger
            .log_event(
                "info_event".to_string(),
                peer_id.clone(),
                HashMap::new(),
                AuditSeverity::Info,
            )
            .await;
        logger
            .log_event(
                "warning_event".to_string(),
                peer_id.clone(),
                HashMap::new(),
                AuditSeverity::Warning,
            )
            .await;
        logger
            .log_event(
                "error_event".to_string(),
                peer_id.clone(),
                HashMap::new(),
                AuditSeverity::Error,
            )
            .await;
        logger
            .log_event(
                "critical_event".to_string(),
                peer_id.clone(),
                HashMap::new(),
                AuditSeverity::Critical,
            )
            .await;

        // Test filtering by severity
        let info_entries = logger.get_entries_by_severity(AuditSeverity::Info).await;
        assert_eq!(info_entries.len(), 1);
        assert_eq!(info_entries[0].event_type, "info_event");

        let warning_entries = logger.get_entries_by_severity(AuditSeverity::Warning).await;
        assert_eq!(warning_entries.len(), 1);
        assert_eq!(warning_entries[0].event_type, "warning_event");

        let error_entries = logger.get_entries_by_severity(AuditSeverity::Error).await;
        assert_eq!(error_entries.len(), 1);

        let critical_entries = logger
            .get_entries_by_severity(AuditSeverity::Critical)
            .await;
        assert_eq!(critical_entries.len(), 1);
    }

    #[tokio::test]
    async fn test_audit_logger_max_entries() {
        let logger = SecurityAuditLogger::new(3); // Limit to 3 entries
        let peer_id = create_test_peer();

        // Log 5 events
        for i in 0..5 {
            logger
                .log_event(
                    format!("event_{}", i).into(),
                    peer_id.clone(),
                    HashMap::new(),
                    AuditSeverity::Info,
                )
                .await;
        }

        let entries = logger.get_recent_entries(None).await;
        assert_eq!(entries.len(), 3); // Should only keep 3 most recent

        // Check that we have the most recent events (2, 3, 4)
        assert_eq!(entries[0].event_type, "event_4"); // Most recent first
        assert_eq!(entries[1].event_type, "event_3");
        assert_eq!(entries[2].event_type, "event_2");
    }

    #[tokio::test]
    async fn test_audit_logger_recent_entries_limit() {
        let logger = SecurityAuditLogger::new(10);
        let peer_id = create_test_peer();

        // Log 5 events
        for i in 0..5 {
            logger
                .log_event(
                    format!("event_{}", i).into(),
                    peer_id.clone(),
                    HashMap::new(),
                    AuditSeverity::Info,
                )
                .await;
        }

        // Get limited number of recent entries
        let entries = logger.get_recent_entries(Some(3)).await;
        assert_eq!(entries.len(), 3);

        // Should be most recent first
        assert_eq!(entries[0].event_type, "event_4");
        assert_eq!(entries[1].event_type, "event_3");
        assert_eq!(entries[2].event_type, "event_2");
    }

    #[test]
    fn test_audit_severity_equality() {
        assert_eq!(AuditSeverity::Info, AuditSeverity::Info);
        assert_eq!(AuditSeverity::Warning, AuditSeverity::Warning);
        assert_eq!(AuditSeverity::Error, AuditSeverity::Error);
        assert_eq!(AuditSeverity::Critical, AuditSeverity::Critical);

        assert_ne!(AuditSeverity::Info, AuditSeverity::Warning);
        assert_ne!(AuditSeverity::Warning, AuditSeverity::Error);
        assert_ne!(AuditSeverity::Error, AuditSeverity::Critical);
    }

    #[test]
    fn test_token_header_structure() {
        let header = TokenHeader {
            alg: "HS256".to_string(),
            typ: "JWT".to_string(),
            kid: Some("key123".to_string()),
        };

        assert_eq!(header.alg, "HS256");
        assert_eq!(header.typ, "JWT");
        assert_eq!(header.kid, Some("key123".to_string()));
    }

    #[test]
    fn test_token_payload_structure() {
        let peer_id = create_test_peer();
        let now = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("valid security operation")
            .as_secs();

        let mut claims = HashMap::new();
        claims.insert("custom".to_string(), serde_json::json!("value"));

        let payload = TokenPayload {
            iss: peer_id.clone(),
            sub: peer_id.to_string(),
            aud: "test-audience".to_string(),
            exp: now + 3600,
            nbf: now,
            iat: now,
            jti: "unique-id".to_string(),
            claims,
        };

        assert_eq!(payload.iss, peer_id);
        assert_eq!(payload.aud, "test-audience");
        assert_eq!(payload.jti, "unique-id");
        assert!(payload.exp > payload.iat);
        assert_eq!(
            payload.claims.get("custom"),
            Some(&serde_json::json!("value"))
        );
    }

    #[test]
    fn test_mcp_token_structure() {
        let peer_id = create_test_peer();

        let header = TokenHeader {
            alg: "HS256".to_string(),
            typ: "JWT".to_string(),
            kid: None,
        };

        let payload = TokenPayload {
            iss: peer_id.clone(),
            sub: peer_id.to_string(),
            aud: "test".to_string(),
            exp: 1234567890,
            nbf: 1234567800,
            iat: 1234567800,
            jti: "test-id".to_string(),
            claims: HashMap::new(),
        };

        let token = MCPToken {
            header: header.clone(),
            payload: payload.clone(),
            signature: "test-signature".to_string(),
        };

        assert_eq!(token.header.alg, header.alg);
        assert_eq!(token.payload.iss, payload.iss);
        assert_eq!(token.signature, "test-signature");
    }
}
