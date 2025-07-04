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

//! MCP Remote Functionality Tests
//!
//! Tests for MCP message routing over P2P network, including remote tool discovery,
//! remote tool execution, and service advertisement functionality.

use p2p_foundation::{P2PNode, NodeConfig, Result};
use p2p_foundation::mcp::{Tool, ToolHandler, P2PMCPMessage, P2PMCPMessageType, MCPMessage};
use serde_json::{json, Value};
use std::sync::Arc;

/// Simple calculator tool for testing
struct CalculatorTool;

impl ToolHandler for CalculatorTool {
    fn execute(&self, arguments: Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>> + Send + '_>> {
        Box::pin(async move {
            let a = arguments.get("a").and_then(|v| v.as_f64())
                .ok_or_else(|| p2p_foundation::P2PError::MCP("Missing or invalid parameter 'a'".to_string()))?;
            let b = arguments.get("b").and_then(|v| v.as_f64())
                .ok_or_else(|| p2p_foundation::P2PError::MCP("Missing or invalid parameter 'b'".to_string()))?;
            let operation = arguments.get("operation").and_then(|v| v.as_str())
                .ok_or_else(|| p2p_foundation::P2PError::MCP("Missing or invalid parameter 'operation'".to_string()))?;
            
            let result = match operation {
                "add" => a + b,
                "subtract" => a - b,
                "multiply" => a * b,
                "divide" => {
                    if b == 0.0 {
                        return Err(p2p_foundation::P2PError::MCP("Division by zero".to_string()));
                    }
                    a / b
                }
                _ => return Err(p2p_foundation::P2PError::MCP(format!("Unknown operation: {}", operation))),
            };
            
            Ok(json!({"result": result}))
        })
    }
    
    fn validate(&self, arguments: &Value) -> Result<()> {
        if !arguments.is_object() {
            return Err(p2p_foundation::P2PError::MCP("Arguments must be an object".to_string()));
        }
        
        if arguments.get("a").is_none() || arguments.get("b").is_none() || arguments.get("operation").is_none() {
            return Err(p2p_foundation::P2PError::MCP("Missing required parameters".to_string()));
        }
        
        Ok(())
    }
}

/// Helper function to create a test P2P node with MCP enabled
async fn create_test_node_with_mcp(peer_id: &str) -> Result<Arc<P2PNode>> {
    // Create MCP config with authentication and rate limiting disabled for testing
    let mut mcp_config = p2p_foundation::mcp::MCPServerConfig::default();
    mcp_config.enable_auth = false;         // Disable authentication for testing
    mcp_config.enable_rate_limiting = false; // Disable rate limiting for testing
    
    let config = NodeConfig {
        peer_id: Some(peer_id.to_string()),
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

/// Test MCP message serialization and deserialization
#[tokio::test]
async fn test_mcp_message_serialization() -> Result<()> {
    // Create a sample P2P MCP message
    let message = P2PMCPMessage {
        message_type: P2PMCPMessageType::Request,
        message_id: "test-123".to_string(),
        source_peer: "peer1".to_string(),
        target_peer: Some("peer2".to_string()),
        timestamp: 1234567890,
        payload: MCPMessage::CallTool {
            name: "calculator".to_string(),
            arguments: json!({
                "a": 5.0,
                "b": 3.0,
                "operation": "add"
            }),
        },
        ttl: 5,
    };
    
    // Serialize the message
    let serialized = serde_json::to_vec(&message).unwrap();
    assert!(serialized.len() > 0);
    
    // Deserialize the message
    let deserialized: P2PMCPMessage = serde_json::from_slice(&serialized).unwrap();
    assert_eq!(deserialized.message_id, "test-123");
    assert_eq!(deserialized.source_peer, "peer1");
    assert_eq!(deserialized.target_peer, Some("peer2".to_string()));
    
    match deserialized.payload {
        MCPMessage::CallTool { name, arguments } => {
            assert_eq!(name, "calculator");
            assert_eq!(arguments["a"], 5.0);
            assert_eq!(arguments["b"], 3.0);
            assert_eq!(arguments["operation"], "add");
        }
        _ => panic!("Unexpected message type"),
    }
    
    Ok(())
}

/// Test MCP server message handling
#[tokio::test]
async fn test_mcp_server_message_handling() -> Result<()> {
    let node = create_test_node_with_mcp("test_node").await?;
    node.start().await?;
    
    // Register a calculator tool
    let calculator_tool = Tool::new(
        "calculator",
        "Basic arithmetic calculator",
        json!({
            "type": "object",
            "properties": {
                "a": {"type": "number"},
                "b": {"type": "number"},
                "operation": {"type": "string", "enum": ["add", "subtract", "multiply", "divide"]}
            },
            "required": ["a", "b", "operation"]
        })
    ).handler(CalculatorTool).build()?;
    
    node.register_mcp_tool(calculator_tool).await?;
    
    // Create a P2P MCP request message
    let request_message = P2PMCPMessage {
        message_type: P2PMCPMessageType::Request,
        message_id: "test-request-123".to_string(),
        source_peer: "remote_peer".to_string(),
        target_peer: Some("test_node".to_string()),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        payload: MCPMessage::CallTool {
            name: "calculator".to_string(),
            arguments: json!({
                "a": 10.0,
                "b": 5.0,
                "operation": "multiply"
            }),
        },
        ttl: 5,
    };
    
    // Serialize the request
    let request_data = serde_json::to_vec(&request_message).unwrap();
    
    // Have the MCP server handle the message
    if let Some(mcp_server) = node.mcp_server() {
        let response_data = mcp_server.handle_p2p_message(&request_data, &"remote_peer".to_string()).await?;
        
        // Should return a response
        assert!(response_data.is_some());
        
        // Deserialize and check the response
        let response_bytes = response_data.unwrap();
        let response: P2PMCPMessage = serde_json::from_slice(&response_bytes).unwrap();
        
        assert_eq!(response.message_type, P2PMCPMessageType::Response);
        assert_eq!(response.message_id, "test-request-123");
        
        match response.payload {
            MCPMessage::CallToolResult { content, is_error } => {
                assert!(!is_error);
                assert!(!content.is_empty());
            }
            _ => panic!("Expected CallToolResult response"),
        }
    } else {
        panic!("MCP server should be available");
    }
    
    node.stop().await?;
    Ok(())
}

/// Test remote tool listing functionality
#[tokio::test]
async fn test_remote_tool_listing() -> Result<()> {
    let node = create_test_node_with_mcp("test_node").await?;
    node.start().await?;
    
    // Register some tools
    let calculator_tool = Tool::new(
        "calculator",
        "Basic arithmetic calculator",
        json!({
            "type": "object",
            "properties": {
                "a": {"type": "number"},
                "b": {"type": "number"},
                "operation": {"type": "string"}
            },
            "required": ["a", "b", "operation"]
        })
    ).handler(CalculatorTool).build()?;
    
    node.register_mcp_tool(calculator_tool).await?;
    
    // First connect to a simulated peer
    let fake_address = "/ip4/127.0.0.1/tcp/9999";
    let connected_peer_id = node.connect_peer(&fake_address.to_string()).await?;
    
    // Test listing remote tools (simulated)
    let remote_tools = node.list_remote_mcp_tools(&connected_peer_id).await?;
    
    // For now, this returns local tools as simulation
    assert!(remote_tools.contains(&"calculator".to_string()));
    
    node.stop().await?;
    Ok(())
}

/// Test remote tool calling functionality
#[tokio::test]
async fn test_remote_tool_calling() -> Result<()> {
    let node = create_test_node_with_mcp("test_node").await?;
    node.start().await?;
    
    // Register a calculator tool locally (to simulate remote)
    let calculator_tool = Tool::new(
        "calculator",
        "Basic arithmetic calculator",
        json!({
            "type": "object",
            "properties": {
                "a": {"type": "number"},
                "b": {"type": "number"},
                "operation": {"type": "string"}
            },
            "required": ["a", "b", "operation"]
        })
    ).handler(CalculatorTool).build()?;
    
    node.register_mcp_tool(calculator_tool).await?;
    
    // Test calling a remote tool (simulated as local for now)
    let remote_peer_id = "remote_peer_456";
    let result = node.call_remote_mcp_tool(
        &remote_peer_id.to_string(), 
        "calculator", 
        json!({
            "a": 15.0,
            "b": 3.0,
            "operation": "divide"
        })
    ).await?;
    
    assert_eq!(result["result"], 5.0);
    
    node.stop().await?;
    Ok(())
}

/// Test service discovery functionality
#[tokio::test]
async fn test_service_discovery() -> Result<()> {
    let node = create_test_node_with_mcp("test_node").await?;
    node.start().await?;
    
    // Test discovering remote services
    let services = node.discover_remote_mcp_services().await?;
    
    // Should return empty list since no services are advertised yet
    assert_eq!(services.len(), 0);
    
    node.stop().await?;
    Ok(())
}

/// Test error handling in remote calls
#[tokio::test]
async fn test_remote_error_handling() -> Result<()> {
    let node = create_test_node_with_mcp("test_node").await?;
    node.start().await?;
    
    // Register a calculator tool
    let calculator_tool = Tool::new(
        "calculator",
        "Basic arithmetic calculator",
        json!({
            "type": "object",
            "properties": {
                "a": {"type": "number"},
                "b": {"type": "number"},
                "operation": {"type": "string"}
            },
            "required": ["a", "b", "operation"]
        })
    ).handler(CalculatorTool).build()?;
    
    node.register_mcp_tool(calculator_tool).await?;
    
    // Test calling a remote tool with invalid parameters (division by zero)
    let remote_peer_id = "remote_peer_error";
    let result = node.call_remote_mcp_tool(
        &remote_peer_id.to_string(), 
        "calculator", 
        json!({
            "a": 10.0,
            "b": 0.0,
            "operation": "divide"
        })
    ).await;
    
    // Should return an error for division by zero
    assert!(result.is_err());
    
    node.stop().await?;
    Ok(())
}

/// Test MCP protocol message types
#[tokio::test]
async fn test_mcp_protocol_message_types() -> Result<()> {
    // Test all message types can be serialized/deserialized
    let test_cases = vec![
        P2PMCPMessageType::Request,
        P2PMCPMessageType::Response,
        P2PMCPMessageType::ServiceAdvertisement,
        P2PMCPMessageType::ServiceDiscovery,
    ];
    
    for message_type in test_cases {
        let message = P2PMCPMessage {
            message_type: message_type.clone(),
            message_id: "test".to_string(),
            source_peer: "peer1".to_string(),
            target_peer: Some("peer2".to_string()),
            timestamp: 1234567890,
            payload: MCPMessage::ListTools { cursor: None },
            ttl: 5,
        };
        
        let serialized = serde_json::to_vec(&message).unwrap();
        let deserialized: P2PMCPMessage = serde_json::from_slice(&serialized).unwrap();
        
        // Verify the message type is preserved
        match (&message_type, &deserialized.message_type) {
            (P2PMCPMessageType::Request, P2PMCPMessageType::Request) => {},
            (P2PMCPMessageType::Response, P2PMCPMessageType::Response) => {},
            (P2PMCPMessageType::ServiceAdvertisement, P2PMCPMessageType::ServiceAdvertisement) => {},
            (P2PMCPMessageType::ServiceDiscovery, P2PMCPMessageType::ServiceDiscovery) => {},
            _ => panic!("Message type not preserved during serialization"),
        }
    }
    
    Ok(())
}

/// Test large message handling
#[tokio::test]
async fn test_large_message_handling() -> Result<()> {
    let node = create_test_node_with_mcp("test_node").await?;
    node.start().await?;
    
    // Create a message with a large payload
    let large_data = "x".repeat(1000); // 1KB of data
    let message = P2PMCPMessage {
        message_type: P2PMCPMessageType::Request,
        message_id: "large-test".to_string(),
        source_peer: "peer1".to_string(),
        target_peer: Some("test_node".to_string()),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        payload: MCPMessage::CallTool {
            name: "echo".to_string(),
            arguments: json!({
                "message": large_data
            }),
        },
        ttl: 5,
    };
    
    // Serialize the message
    let message_data = serde_json::to_vec(&message).unwrap();
    
    // Verify it's within size limits
    assert!(message_data.len() < p2p_foundation::mcp::MAX_MESSAGE_SIZE);
    
    node.stop().await?;
    Ok(())
}