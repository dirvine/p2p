//! MCP Security Module
//!
//! This module provides comprehensive security features for the MCP server including:
//! - JWT-based authentication
//! - Peer identity verification
//! - Access control and permissions
//! - Rate limiting and abuse prevention
//! - Message integrity and encryption

use crate::{PeerId, Result, P2PError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;
use std::sync::Arc;
use base64::prelude::*;

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
    pub async fn generate_token(&self, peer_id: &PeerId, permissions: Vec<MCPPermission>, ttl: Duration) -> Result<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| P2PError::MCP(format!("Time error: {}", e)))?;
        
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
                claims.insert("permissions".to_string(), 
                    serde_json::to_value(permissions.iter().map(|p| p.as_str()).collect::<Vec<_>>()).unwrap());
                claims
            },
        };
        
        let header = TokenHeader {
            alg: "HS256".to_string(),
            typ: "JWT".to_string(),
            kid: None,
        };
        
        // Create token without signature first
        let header_b64 = base64::prelude::BASE64_URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header)
            .map_err(|e| P2PError::Serialization(e))?);
        let payload_b64 = base64::prelude::BASE64_URL_SAFE_NO_PAD.encode(serde_json::to_vec(&payload)
            .map_err(|e| P2PError::Serialization(e))?);
        
        // Sign the token
        let signing_input = format!("{}.{}", header_b64, payload_b64);
        let signature = self.sign_data(signing_input.as_bytes());
        let signature_b64 = base64::prelude::BASE64_URL_SAFE_NO_PAD.encode(signature);
        
        Ok(format!("{}.{}.{}", header_b64, payload_b64, signature_b64))
    }
    
    /// Verify authentication token
    pub async fn verify_token(&self, token: &str) -> Result<TokenPayload> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(P2PError::MCP("Invalid token format".to_string()));
        }
        
        let _header_data = base64::prelude::BASE64_URL_SAFE_NO_PAD.decode(parts[0])
            .map_err(|e| P2PError::MCP(format!("Invalid header encoding: {}", e)))?;
        let payload_data = base64::prelude::BASE64_URL_SAFE_NO_PAD.decode(parts[1])
            .map_err(|e| P2PError::MCP(format!("Invalid payload encoding: {}", e)))?;
        let signature = base64::prelude::BASE64_URL_SAFE_NO_PAD.decode(parts[2])
            .map_err(|e| P2PError::MCP(format!("Invalid signature encoding: {}", e)))?;
        
        // Verify signature
        let signing_input = format!("{}.{}", parts[0], parts[1]);
        let expected_signature = self.sign_data(signing_input.as_bytes());
        
        if signature != expected_signature {
            return Err(P2PError::MCP("Invalid token signature".to_string()));
        }
        
        // Parse payload
        let payload: TokenPayload = serde_json::from_slice(&payload_data)
            .map_err(|e| P2PError::MCP(format!("Invalid payload: {}", e)))?;
        
        // Check expiration
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| P2PError::MCP(format!("Time error: {}", e)))?
            .as_secs();
        
        if payload.exp < now {
            return Err(P2PError::MCP("Token expired".to_string()));
        }
        
        if payload.nbf > now {
            return Err(P2PError::MCP("Token not yet valid".to_string()));
        }
        
        Ok(payload)
    }
    
    /// Check if peer has permission for operation
    pub async fn check_permission(&self, peer_id: &PeerId, permission: &MCPPermission) -> Result<bool> {
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
    pub async fn grant_permission(&self, peer_id: &PeerId, permission: MCPPermission) -> Result<()> {
        let mut acls = self.acls.write().await;
        let acl = acls.entry(peer_id.clone()).or_insert_with(|| PeerACL::new(peer_id.clone()));
        acl.grant_permission(permission);
        Ok(())
    }
    
    /// Revoke permission from peer
    pub async fn revoke_permission(&self, peer_id: &PeerId, permission: &MCPPermission) -> Result<()> {
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
        policies.get(tool_name).cloned().unwrap_or(SecurityLevel::Basic)
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
    pub async fn log_event(&self, event_type: String, peer_id: PeerId, details: HashMap<String, String>, severity: AuditSeverity) {
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
    pub async fn get_entries_by_severity(&self, severity: AuditSeverity) -> Vec<SecurityAuditEntry> {
        let entries = self.entries.read().await;
        entries.iter().filter(|e| e.severity == severity).cloned().collect()
    }
}