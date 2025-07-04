
//! MCP Integration Tests
//!
//! Comprehensive tests for the Model Context Protocol (MCP) server integration
//! with the P2P network, including tool registration, discovery, and execution.

use p2p_foundation::{P2PNode, NodeConfig, Result};
use p2p_foundation::mcp::{Tool, FunctionToolHandler, ToolHandler};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;

/// Helper function to create a test P2P node with MCP enabled
async fn create_test_node_with_mcp() -> Result<Arc<P2PNode>> {
    let mut mcp_config = p2p_foundation::mcp::MCPServerConfig::default();
    // Disable authentication for testing to simplify test setup
    mcp_config.enable_auth = false;
    mcp_config.enable_rate_limiting = false;
    
    let config = NodeConfig {
        peer_id: Some(format!("test_node_{}", uuid::Uuid::new_v4().to_string()[..8].to_string())),
        listen_addrs: vec![
            format!("/ip4/127.0.0.1/tcp/{}", 9000 + rand::random::<u16>() % 1000)
        ],
        enable_mcp_server: true,
        mcp_server_config: Some(mcp_config),
        ..NodeConfig::default()
    };
    
    let node = P2PNode::new(config).await?;
    let node = Arc::new(node);
    
    Ok(node)
}

/// Simple calculator tool for testing
struct CalculatorTool;

impl ToolHandler for CalculatorTool {
    fn execute(&self, arguments: Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>> + Send + '_>> {
        Box::pin(async move {
            let a = arguments.get("a").and_then(|v| v.as_f64())
                .ok_or_else(|| p2p_foundation::P2PError::MCP("Missing or invalid parameter 'a'".to_string()))?;
            let b = arguments.get("b").and_then(|v| v.as_f64())
                .ok_or_else(|| p2p_foundation::P2PError::MCP("Missing or invalid parameter 'b'".to_string()))?;
            let op = arguments.get("operation").and_then(|v| v.as_str())
                .ok_or_else(|| p2p_foundation::P2PError::MCP("Missing or invalid parameter 'operation'".to_string()))?;
            
            let result = match op {
                "add" => a + b,
                "subtract" => a - b,
                "multiply" => a * b,
                "divide" => {
                    if b == 0.0 {
                        return Err(p2p_foundation::P2PError::MCP("Division by zero".to_string()));
                    }
                    a / b
                }
                _ => return Err(p2p_foundation::P2PError::MCP(format!("Unknown operation: {}", op))),
            };
            
            Ok(json!({"result": result}))
        })
    }
    
    fn validate(&self, arguments: &Value) -> Result<()> {
        // Check required parameters
        if !arguments.is_object() {
            return Err(p2p_foundation::P2PError::MCP("Arguments must be an object".to_string()));
        }
        
        if arguments.get("a").is_none() || arguments.get("b").is_none() || arguments.get("operation").is_none() {
            return Err(p2p_foundation::P2PError::MCP("Missing required parameters".to_string()));
        }
        
        Ok(())
    }
}

/// Test MCP server creation and basic functionality
#[tokio::test]
async fn test_mcp_server_creation() -> Result<()> {
    let node = create_test_node_with_mcp().await?;
    
    // Verify MCP server is available
    assert!(node.mcp_server().is_some());
    
    // Start the node
    node.start().await?;
    
    // Verify it's running
    assert!(node.is_running().await);
    
    // Stop the node
    node.stop().await?;
    
    // Verify it's stopped
    assert!(!node.is_running().await);
    
    Ok(())
}

/// Test tool registration and listing
#[tokio::test]
async fn test_tool_registration() -> Result<()> {
    let node = create_test_node_with_mcp().await?;
    node.start().await?;
    
    // Initially no tools
    let tools = node.list_mcp_tools().await?;
    assert_eq!(tools.len(), 0);
    
    // Create and register a calculator tool
    let calculator_tool = Tool::new(
        "calculator",
        "Basic arithmetic calculator",
        json!({
            "type": "object",
            "properties": {
                "a": {"type": "number", "description": "First number"},
                "b": {"type": "number", "description": "Second number"},
                "operation": {
                    "type": "string",
                    "enum": ["add", "subtract", "multiply", "divide"],
                    "description": "Arithmetic operation to perform"
                }
            },
            "required": ["a", "b", "operation"]
        })
    ).handler(CalculatorTool).build()?;
    
    node.register_mcp_tool(calculator_tool).await?;
    
    // Verify tool is registered
    let tools = node.list_mcp_tools().await?;
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0], "calculator");
    
    node.stop().await?;
    Ok(())
}

/// Test tool execution
#[tokio::test]
async fn test_tool_execution() -> Result<()> {
    let node = create_test_node_with_mcp().await?;
    node.start().await?;
    
    // Register calculator tool
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
    
    // Test addition
    let result = node.call_mcp_tool("calculator", json!({
        "a": 5.0,
        "b": 3.0,
        "operation": "add"
    })).await?;
    
    assert_eq!(result["result"], 8.0);
    
    // Test multiplication
    let result = node.call_mcp_tool("calculator", json!({
        "a": 4.0,
        "b": 7.0,
        "operation": "multiply"
    })).await?;
    
    assert_eq!(result["result"], 28.0);
    
    // Test division
    let result = node.call_mcp_tool("calculator", json!({
        "a": 10.0,
        "b": 2.0,
        "operation": "divide"
    })).await?;
    
    assert_eq!(result["result"], 5.0);
    
    node.stop().await?;
    Ok(())
}

/// Test tool validation
#[tokio::test]
async fn test_tool_validation() -> Result<()> {
    let node = create_test_node_with_mcp().await?;
    node.start().await?;
    
    // Register calculator tool
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
    
    // Test with missing parameters
    let result = node.call_mcp_tool("calculator", json!({"a": 5.0})).await;
    assert!(result.is_err());
    
    // Test with invalid operation
    let result = node.call_mcp_tool("calculator", json!({
        "a": 5.0,
        "b": 3.0,
        "operation": "invalid_op"
    })).await;
    assert!(result.is_err());
    
    // Test division by zero
    let result = node.call_mcp_tool("calculator", json!({
        "a": 5.0,
        "b": 0.0,
        "operation": "divide"
    })).await;
    assert!(result.is_err());
    
    node.stop().await?;
    Ok(())
}

/// Test function-based tool handler
#[tokio::test]
async fn test_function_tool_handler() -> Result<()> {
    let node = create_test_node_with_mcp().await?;
    node.start().await?;
    
    // Create a simple echo tool using function handler
    let echo_handler = FunctionToolHandler::new(|args: Value| async move {
        let message = args.get("message").and_then(|v| v.as_str())
            .ok_or_else(|| p2p_foundation::P2PError::MCP("Missing message parameter".to_string()))?;
        Ok(json!({"echo": message}))
    });
    
    let echo_tool = Tool::new(
        "echo",
        "Echo tool that returns the input message",
        json!({
            "type": "object",
            "properties": {
                "message": {"type": "string", "description": "Message to echo back"}
            },
            "required": ["message"]
        })
    ).handler(echo_handler).build()?;
    
    node.register_mcp_tool(echo_tool).await?;
    
    // Test echo functionality
    let result = node.call_mcp_tool("echo", json!({
        "message": "Hello, MCP!"
    })).await?;
    
    assert_eq!(result["echo"], "Hello, MCP!");
    
    node.stop().await?;
    Ok(())
}

/// Test multiple tools registration and execution
#[tokio::test]
async fn test_multiple_tools() -> Result<()> {
    let node = create_test_node_with_mcp().await?;
    node.start().await?;
    
    // Register calculator tool
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
    
    // Register echo tool
    let echo_handler = FunctionToolHandler::new(|args: Value| async move {
        let message = args.get("message").and_then(|v| v.as_str())
            .ok_or_else(|| p2p_foundation::P2PError::MCP("Missing message parameter".to_string()))?;
        Ok(json!({"echo": message}))
    });
    
    let echo_tool = Tool::new(
        "echo",
        "Echo tool",
        json!({
            "type": "object",
            "properties": {
                "message": {"type": "string"}
            },
            "required": ["message"]
        })
    ).handler(echo_handler).build()?;
    
    node.register_mcp_tool(echo_tool).await?;
    
    // Register timestamp tool
    let timestamp_handler = FunctionToolHandler::new(|_args: Value| async move {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Ok(json!({"timestamp": now}))
    });
    
    let timestamp_tool = Tool::new(
        "timestamp",
        "Get current timestamp",
        json!({
            "type": "object",
            "properties": {}
        })
    ).handler(timestamp_handler).build()?;
    
    node.register_mcp_tool(timestamp_tool).await?;
    
    // Verify all tools are registered
    let tools = node.list_mcp_tools().await?;
    assert_eq!(tools.len(), 3);
    assert!(tools.contains(&"calculator".to_string()));
    assert!(tools.contains(&"echo".to_string()));
    assert!(tools.contains(&"timestamp".to_string()));
    
    // Test each tool
    let calc_result = node.call_mcp_tool("calculator", json!({
        "a": 6.0,
        "b": 2.0,
        "operation": "multiply"
    })).await?;
    assert_eq!(calc_result["result"], 12.0);
    
    let echo_result = node.call_mcp_tool("echo", json!({
        "message": "Test message"
    })).await?;
    assert_eq!(echo_result["echo"], "Test message");
    
    let timestamp_result = node.call_mcp_tool("timestamp", json!({})).await?;
    assert!(timestamp_result["timestamp"].is_number());
    
    node.stop().await?;
    Ok(())
}

/// Test MCP server statistics
#[tokio::test]
async fn test_mcp_statistics() -> Result<()> {
    let node = create_test_node_with_mcp().await?;
    node.start().await?;
    
    // Register a simple tool
    let echo_handler = FunctionToolHandler::new(|args: Value| async move {
        let message = args.get("message").and_then(|v| v.as_str())
            .ok_or_else(|| p2p_foundation::P2PError::MCP("Missing message parameter".to_string()))?;
        Ok(json!({"echo": message}))
    });
    
    let echo_tool = Tool::new(
        "echo",
        "Echo tool",
        json!({
            "type": "object",
            "properties": {
                "message": {"type": "string"}
            },
            "required": ["message"]
        })
    ).handler(echo_handler).build()?;
    
    node.register_mcp_tool(echo_tool).await?;
    
    // Get initial stats
    let initial_stats = node.mcp_stats().await?;
    assert_eq!(initial_stats.total_requests, 0);
    assert_eq!(initial_stats.total_tools, 1);
    
    // Call the tool a few times
    for i in 0..3 {
        node.call_mcp_tool("echo", json!({
            "message": format!("Message {}", i)
        })).await?;
    }
    
    // Get updated stats
    let updated_stats = node.mcp_stats().await?;
    assert_eq!(updated_stats.total_requests, 3);
    assert_eq!(updated_stats.total_responses, 3);
    assert_eq!(updated_stats.total_tools, 1);
    assert!(updated_stats.popular_tools.contains_key("echo"));
    assert_eq!(updated_stats.popular_tools["echo"], 3);
    
    node.stop().await?;
    Ok(())
}

/// Test tool error handling
#[tokio::test]
async fn test_tool_error_handling() -> Result<()> {
    let node = create_test_node_with_mcp().await?;
    node.start().await?;
    
    // Create a tool that always fails
    let error_handler = FunctionToolHandler::new(|_args: Value| async move {
        Err(p2p_foundation::P2PError::MCP("This tool always fails".to_string()))
    });
    
    let error_tool = Tool::new(
        "error_tool",
        "Tool that always errors",
        json!({
            "type": "object",
            "properties": {}
        })
    ).handler(error_handler).build()?;
    
    node.register_mcp_tool(error_tool).await?;
    
    // Call the error tool
    let result = node.call_mcp_tool("error_tool", json!({})).await;
    assert!(result.is_err());
    
    // Call a non-existent tool
    let result = node.call_mcp_tool("non_existent", json!({})).await;
    assert!(result.is_err());
    
    node.stop().await?;
    Ok(())
}

/// Test concurrent tool calls
#[tokio::test]
async fn test_concurrent_tool_calls() -> Result<()> {
    let node = create_test_node_with_mcp().await?;
    node.start().await?;
    
    // Register a slow tool
    let slow_handler = FunctionToolHandler::new(|args: Value| async move {
        let delay_ms = args.get("delay_ms").and_then(|v| v.as_u64()).unwrap_or(100);
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        Ok(json!({"completed": true}))
    });
    
    let slow_tool = Tool::new(
        "slow_tool",
        "Tool with configurable delay",
        json!({
            "type": "object",
            "properties": {
                "delay_ms": {"type": "number", "description": "Delay in milliseconds"}
            }
        })
    ).handler(slow_handler).build()?;
    
    node.register_mcp_tool(slow_tool).await?;
    
    // Make multiple concurrent calls
    let mut handles = Vec::new();
    for _i in 0..5 {
        let node_clone = node.clone();
        let handle = tokio::spawn(async move {
            node_clone.call_mcp_tool("slow_tool", json!({
                "delay_ms": 50
            })).await
        });
        handles.push(handle);
    }
    
    // Wait for all calls to complete
    let mut success_count = 0;
    for handle in handles {
        match handle.await {
            Ok(Ok(_)) => success_count += 1,
            _ => {}
        }
    }
    
    // All calls should succeed
    assert_eq!(success_count, 5);
    
    node.stop().await?;
    Ok(())
}

/// Test node builder with MCP configuration
#[tokio::test]
async fn test_node_builder_mcp() -> Result<()> {
    let mut mcp_config = p2p_foundation::mcp::MCPServerConfig::default();
    mcp_config.enable_auth = false;
    mcp_config.enable_rate_limiting = false;
    
    let node = Arc::new(P2PNode::builder()
        .with_peer_id("test_mcp_node".to_string())
        .listen_on("/ip4/127.0.0.1/tcp/9876")
        .with_mcp_config(mcp_config)
        .build()
        .await?);
    
    // Verify MCP server is available
    assert!(node.mcp_server().is_some());
    assert!(node.config().enable_mcp_server);
    
    // Start and test basic functionality
    node.start().await?;
    
    // Register a simple tool
    let echo_handler = FunctionToolHandler::new(|args: Value| async move {
        Ok(json!({"received": args}))
    });
    
    let echo_tool = Tool::new(
        "echo",
        "Echo tool",
        json!({"type": "object"})
    ).handler(echo_handler).build()?;
    
    node.register_mcp_tool(echo_tool).await?;
    
    // Test tool call
    let result = node.call_mcp_tool("echo", json!({"test": "data"})).await?;
    assert_eq!(result["received"]["test"], "data");
    
    node.stop().await?;
    Ok(())
}