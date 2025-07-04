// Copyright 2024 MaidSafe Limited
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

//! MCP Security Tests
//!
//! Comprehensive tests for MCP security features including authentication,
//! authorization, rate limiting, and audit logging.

use p2p_foundation::{P2PNode, NodeConfig, Result};
use p2p_foundation::mcp::{Tool, FunctionToolHandler, ToolHandler, MCPPermission, SecurityLevel};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;

/// Helper function to create a test P2P node with MCP security enabled
async fn create_secure_mcp_node() -> Result<Arc<P2PNode>> {
    let mcp_config = p2p_foundation::mcp::MCPServerConfig {
        enable_auth: true,
        enable_rate_limiting: true,
        rate_limit_rpm: 10, // Low limit for testing
        ..p2p_foundation::mcp::MCPServerConfig::default()
    };
    
    let config = NodeConfig {
        peer_id: Some(format!("secure_node_{}", uuid::Uuid::new_v4().to_string()[..8].to_string())),
        listen_addrs: vec![
            format!("/ip4/127.0.0.1/tcp/{}", 9000 + rand::random::<u16>() % 1000)
        ],
        enable_mcp_server: true,
        mcp_server_config: Some(mcp_config),
        ..NodeConfig::default()
    };
    
    let node = P2PNode::new(config).await?;
    Ok(Arc::new(node))
}

/// Simple calculator tool for testing
struct SecureCalculatorTool;

impl ToolHandler for SecureCalculatorTool {
    fn execute(&self, arguments: Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>> + Send + '_>> {
        Box::pin(async move {
            let a = arguments.get("a").and_then(|v| v.as_f64())
                .ok_or_else(|| p2p_foundation::P2PError::MCP("Missing parameter 'a'".to_string()))?;
            let b = arguments.get("b").and_then(|v| v.as_f64())
                .ok_or_else(|| p2p_foundation::P2PError::MCP("Missing parameter 'b'".to_string()))?;
            
            Ok(json!({"result": a + b}))
        })
    }
    
    fn validate(&self, arguments: &Value) -> Result<()> {
        if !arguments.is_object() {
            return Err(p2p_foundation::P2PError::MCP("Arguments must be an object".to_string()));
        }
        Ok(())
    }
}

/// Test authentication token generation and verification
#[tokio::test]
async fn test_authentication_tokens() -> Result<()> {
    let node = create_secure_mcp_node().await?;
    node.start().await?;
    
    let peer_id = "test_peer";
    let permissions = vec![MCPPermission::ExecuteTools, MCPPermission::ReadTools];
    let ttl = Duration::from_secs(3600);
    
    // Generate authentication token
    if let Some(mcp_server) = node.mcp_server() {
        let token = mcp_server.generate_auth_token(&peer_id.to_string(), permissions.clone(), ttl).await?;
        
        // Verify the token
        let payload = mcp_server.verify_auth_token(&token).await?;
        
        assert_eq!(payload.iss, peer_id);
        assert_eq!(payload.sub, peer_id);
        
        // Check permissions in claims
        if let Some(perms) = payload.claims.get("permissions") {
            let perms_array = perms.as_array().unwrap();
            assert!(perms_array.contains(&json!("execute:tools")));
            assert!(perms_array.contains(&json!("read:tools")));
        }
    }
    
    node.stop().await?;
    Ok(())
}

/// Test permission system
#[tokio::test]
async fn test_permission_system() -> Result<()> {
    let node = create_secure_mcp_node().await?;
    node.start().await?;
    
    let peer_id = "test_peer";
    
    if let Some(mcp_server) = node.mcp_server() {
        // Initially no permissions
        assert!(!mcp_server.check_permission(&peer_id.to_string(), &MCPPermission::ExecuteTools).await?);
        
        // Grant permission
        mcp_server.grant_permission(&peer_id.to_string(), MCPPermission::ExecuteTools).await?;
        
        // Check permission granted
        assert!(mcp_server.check_permission(&peer_id.to_string(), &MCPPermission::ExecuteTools).await?);
        
        // Revoke permission
        mcp_server.revoke_permission(&peer_id.to_string(), &MCPPermission::ExecuteTools).await?;
        
        // Check permission revoked
        assert!(!mcp_server.check_permission(&peer_id.to_string(), &MCPPermission::ExecuteTools).await?);
    }
    
    node.stop().await?;
    Ok(())
}

/// Test rate limiting
#[tokio::test]
async fn test_rate_limiting() -> Result<()> {
    let node = create_secure_mcp_node().await?;
    node.start().await?;
    
    let peer_id = "rate_test_peer";
    
    if let Some(mcp_server) = node.mcp_server() {
        // First few requests should be allowed
        for _i in 0..5 {
            assert!(mcp_server.check_rate_limit(&peer_id.to_string()).await?);
        }
        
        // Exceed rate limit
        for _i in 0..10 {
            mcp_server.check_rate_limit(&peer_id.to_string()).await?;
        }
        
        // Should now be rate limited
        assert!(!mcp_server.check_rate_limit(&peer_id.to_string()).await?);
    }
    
    node.stop().await?;
    Ok(())
}

/// Test tool security policies
#[tokio::test]
async fn test_tool_security_policies() -> Result<()> {
    let node = create_secure_mcp_node().await?;
    node.start().await?;
    
    if let Some(mcp_server) = node.mcp_server() {
        let tool_name = "secure_calculator";
        
        // Set security policy
        mcp_server.set_tool_security_policy(tool_name.to_string(), SecurityLevel::Admin).await?;
        
        // Verify policy
        let policy = mcp_server.get_tool_security_policy(tool_name).await;
        assert_eq!(policy, SecurityLevel::Admin);
        
        // Change policy
        mcp_server.set_tool_security_policy(tool_name.to_string(), SecurityLevel::Basic).await?;
        let policy = mcp_server.get_tool_security_policy(tool_name).await;
        assert_eq!(policy, SecurityLevel::Basic);
    }
    
    node.stop().await?;
    Ok(())
}

/// Test trusted peer system
#[tokio::test]
async fn test_trusted_peers() -> Result<()> {
    let node = create_secure_mcp_node().await?;
    node.start().await?;
    
    let peer_id = "trusted_peer";
    
    if let Some(mcp_server) = node.mcp_server() {
        // Initially not trusted
        assert!(!mcp_server.is_trusted_peer(&peer_id.to_string()).await);
        
        // Add to trusted list
        mcp_server.add_trusted_peer(peer_id.to_string()).await?;
        
        // Should now be trusted
        assert!(mcp_server.is_trusted_peer(&peer_id.to_string()).await);
    }
    
    node.stop().await?;
    Ok(())
}

/// Test security audit logging
#[tokio::test]
async fn test_security_audit_logging() -> Result<()> {
    let node = create_secure_mcp_node().await?;
    node.start().await?;
    
    let peer_id = "audit_test_peer";
    
    if let Some(mcp_server) = node.mcp_server() {
        // Generate some security events
        let _ = mcp_server.generate_auth_token(&peer_id.to_string(), vec![MCPPermission::ReadTools], Duration::from_secs(3600)).await;
        mcp_server.grant_permission(&peer_id.to_string(), MCPPermission::ExecuteTools).await?;
        mcp_server.set_tool_security_policy("test_tool".to_string(), SecurityLevel::Basic).await?;
        
        // Get audit entries
        let entries = mcp_server.get_security_audit(Some(10)).await;
        
        // Should have recorded events
        assert!(entries.len() > 0);
        
        // Check for authentication event
        let auth_events: Vec<_> = entries.iter()
            .filter(|e| e.event_type == "authentication")
            .collect();
        assert!(auth_events.len() > 0);
        
        // Check for authorization event
        let authz_events: Vec<_> = entries.iter()
            .filter(|e| e.event_type == "authorization")
            .collect();
        assert!(authz_events.len() > 0);
    }
    
    node.stop().await?;
    Ok(())
}

/// Test security integration with tool execution
#[tokio::test]
async fn test_secure_tool_execution() -> Result<()> {
    let node = create_secure_mcp_node().await?;
    node.start().await?;
    
    // Register a secure calculator tool
    let calculator_tool = Tool::new(
        "secure_calculator",
        "Secure arithmetic calculator",
        json!({
            "type": "object",
            "properties": {
                "a": {"type": "number"},
                "b": {"type": "number"}
            },
            "required": ["a", "b"]
        })
    ).handler(SecureCalculatorTool).build()?;
    
    node.register_mcp_tool(calculator_tool).await?;
    
    let peer_id = "secure_test_peer";
    
    if let Some(mcp_server) = node.mcp_server() {
        // Set tool security policy to require authentication
        mcp_server.set_tool_security_policy("secure_calculator".to_string(), SecurityLevel::Basic).await?;
        
        // Grant necessary permissions
        mcp_server.grant_permission(&peer_id.to_string(), MCPPermission::ExecuteTools).await?;
        
        // Generate auth token
        let token = mcp_server.generate_auth_token(&peer_id.to_string(), vec![MCPPermission::ExecuteTools], Duration::from_secs(3600)).await?;
        
        // Create authenticated call context
        let context = p2p_foundation::mcp::MCPCallContext {
            caller_id: peer_id.to_string(),
            timestamp: std::time::SystemTime::now(),
            timeout: Duration::from_secs(30),
            auth_info: Some(p2p_foundation::mcp::MCPAuthInfo {
                token: token.clone(),
                token_type: "Bearer".to_string(),
                expires_at: Some(std::time::SystemTime::now() + Duration::from_secs(3600)),
                permissions: vec!["execute:tools".to_string()],
            }),
            metadata: std::collections::HashMap::new(),
        };
        
        // Call should succeed with valid auth
        let result = mcp_server.call_tool("secure_calculator", json!({"a": 5.0, "b": 3.0}), context).await?;
        assert_eq!(result["result"], 8.0);
    }
    
    node.stop().await?;
    Ok(())
}

/// Test security enforcement without authentication
#[tokio::test]
async fn test_security_enforcement_no_auth() -> Result<()> {
    let node = create_secure_mcp_node().await?;
    node.start().await?;
    
    // Register a tool
    let calculator_tool = Tool::new(
        "protected_calculator",
        "Protected calculator",
        json!({
            "type": "object",
            "properties": {
                "a": {"type": "number"},
                "b": {"type": "number"}
            },
            "required": ["a", "b"]
        })
    ).handler(SecureCalculatorTool).build()?;
    
    node.register_mcp_tool(calculator_tool).await?;
    
    let peer_id = "unauthorized_peer";
    
    if let Some(mcp_server) = node.mcp_server() {
        // Set tool to require authentication
        mcp_server.set_tool_security_policy("protected_calculator".to_string(), SecurityLevel::Basic).await?;
        
        // Don't grant permissions or provide auth token
        
        // Create unauthenticated call context
        let context = p2p_foundation::mcp::MCPCallContext {
            caller_id: peer_id.to_string(),
            timestamp: std::time::SystemTime::now(),
            timeout: Duration::from_secs(30),
            auth_info: None, // No authentication
            metadata: std::collections::HashMap::new(),
        };
        
        // Call should fail
        let result = mcp_server.call_tool("protected_calculator", json!({"a": 5.0, "b": 3.0}), context).await;
        assert!(result.is_err());
        
        // Should contain permission or authentication error
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Permission denied") || error_msg.contains("Authentication required"));
    }
    
    node.stop().await?;
    Ok(())
}

/// Test expired token handling
#[tokio::test]
async fn test_expired_token_handling() -> Result<()> {
    let node = create_secure_mcp_node().await?;
    node.start().await?;
    
    let peer_id = "token_test_peer";
    
    if let Some(mcp_server) = node.mcp_server() {
        // Generate token with short TTL (use seconds precision since JWT uses seconds)
        let short_ttl = Duration::from_secs(1);
        let token = mcp_server.generate_auth_token(&peer_id.to_string(), vec![MCPPermission::ReadTools], short_ttl).await?;
        
        // Verify token works initially
        let initial_result = mcp_server.verify_auth_token(&token).await;
        assert!(initial_result.is_ok(), "Token should be valid initially");
        
        // Wait for token to expire (wait longer than TTL)
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // Verification should now fail
        let result = mcp_server.verify_auth_token(&token).await;
        assert!(result.is_err(), "Token should be expired and verification should fail");
        
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.to_lowercase().contains("expired") || error_msg.to_lowercase().contains("invalid"), 
               "Error should indicate token is expired or invalid: {}", error_msg);
    }
    
    node.stop().await?;
    Ok(())
}

/// Test admin-level security
#[tokio::test]
async fn test_admin_security() -> Result<()> {
    let node = create_secure_mcp_node().await?;
    node.start().await?;
    
    // Register an admin-only tool
    let admin_tool = Tool::new(
        "admin_tool",
        "Administrative tool",
        json!({
            "type": "object",
            "properties": {}
        })
    ).handler(FunctionToolHandler::new(|_args: Value| async move {
        Ok(json!({"status": "admin operation completed"}))
    })).build()?;
    
    node.register_mcp_tool(admin_tool).await?;
    
    let regular_peer = "regular_peer";
    let admin_peer = "admin_peer";
    
    if let Some(mcp_server) = node.mcp_server() {
        // Set tool to require admin access
        mcp_server.set_tool_security_policy("admin_tool".to_string(), SecurityLevel::Admin).await?;
        
        // Grant basic permissions to regular peer
        mcp_server.grant_permission(&regular_peer.to_string(), MCPPermission::ExecuteTools).await?;
        
        // Grant admin permissions to admin peer
        mcp_server.grant_permission(&admin_peer.to_string(), MCPPermission::Admin).await?;
        
        // Regular peer should be denied
        assert!(!mcp_server.check_permission(&regular_peer.to_string(), &MCPPermission::Admin).await?);
        
        // Admin peer should be allowed
        assert!(mcp_server.check_permission(&admin_peer.to_string(), &MCPPermission::Admin).await?);
    }
    
    node.stop().await?;
    Ok(())
}

/// Test security cleanup functionality
#[tokio::test]
async fn test_security_cleanup() -> Result<()> {
    let node = create_secure_mcp_node().await?;
    node.start().await?;
    
    if let Some(mcp_server) = node.mcp_server() {
        // Generate some activity
        let peer_id = "cleanup_test_peer";
        mcp_server.check_rate_limit(&peer_id.to_string()).await?;
        mcp_server.grant_permission(&peer_id.to_string(), MCPPermission::ReadTools).await?;
        
        // Perform cleanup
        mcp_server.security_cleanup().await?;
        
        // Cleanup should complete successfully
        // (This mainly tests that the cleanup doesn't crash)
    }
    
    node.stop().await?;
    Ok(())
}