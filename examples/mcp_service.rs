
//! MCP Service Example
//!
//! This example demonstrates how to create and use MCP services with the P2P Foundation.

use p2p_foundation::{P2PNode, Result};
use p2p_foundation::mcp::{Tool, FunctionToolHandler, ToolHandler};
use serde_json::{json, Value};

/// Calculator tool implementation
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

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into())
        )
        .init();

    println!("MCP Service Example");
    println!("==================");
    println!();

    // Create a P2P node with MCP server enabled
    let node = P2PNode::builder()
        .with_peer_id("mcp_example_node".to_string())
        .listen_on("/ip4/127.0.0.1/tcp/9000")
        .with_mcp_server()
        .build()
        .await?;

    println!("Created P2P node with MCP server: {}", node.peer_id());
    
    // Start the node
    node.start().await?;
    println!("Started P2P node with MCP server");
    
    // Register a calculator tool
    let calculator_tool = Tool::new(
        "calculator",
        "Basic arithmetic calculator with four operations",
        json!({
            "type": "object",
            "properties": {
                "a": {
                    "type": "number",
                    "description": "First number for the operation"
                },
                "b": {
                    "type": "number", 
                    "description": "Second number for the operation"
                },
                "operation": {
                    "type": "string",
                    "enum": ["add", "subtract", "multiply", "divide"],
                    "description": "The arithmetic operation to perform"
                }
            },
            "required": ["a", "b", "operation"]
        })
    )
    .handler(CalculatorTool)
    .tags(vec!["math".to_string(), "calculator".to_string()])
    .build()?;

    node.register_mcp_tool(calculator_tool).await?;
    println!("Registered calculator tool");

    // Register a timestamp tool using function handler
    let timestamp_handler = FunctionToolHandler::new(|_args: Value| async move {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| p2p_foundation::P2PError::MCP(format!("Time error: {}", e)))?
            .as_secs();
        
        let formatted_time = chrono::DateTime::from_timestamp(now as i64, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| "Unknown time".to_string());
        
        Ok(json!({
            "timestamp": now,
            "formatted": formatted_time
        }))
    });

    let timestamp_tool = Tool::new(
        "timestamp",
        "Get current Unix timestamp and formatted time",
        json!({
            "type": "object",
            "properties": {},
            "description": "No parameters required"
        })
    )
    .handler(timestamp_handler)
    .tags(vec!["time".to_string(), "utility".to_string()])
    .build()?;

    node.register_mcp_tool(timestamp_tool).await?;
    println!("Registered timestamp tool");

    // Register a greeting tool
    let greeting_handler = FunctionToolHandler::new(|args: Value| async move {
        let name = args.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("World");
        
        let language = args.get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("english");
        
        let greeting = match language.to_lowercase().as_str() {
            "spanish" => format!("¡Hola, {}!", name),
            "french" => format!("Bonjour, {} !", name),
            "german" => format!("Hallo, {}!", name),
            "japanese" => format!("こんにちは、{}さん！", name),
            _ => format!("Hello, {}!", name),
        };
        
        Ok(json!({
            "greeting": greeting,
            "language": language,
            "name": name
        }))
    });

    let greeting_tool = Tool::new(
        "greeting",
        "Generate a greeting in different languages",
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Name of the person to greet",
                    "default": "World"
                },
                "language": {
                    "type": "string",
                    "enum": ["english", "spanish", "french", "german", "japanese"],
                    "description": "Language for the greeting",
                    "default": "english"
                }
            }
        })
    )
    .handler(greeting_handler)
    .tags(vec!["greeting".to_string(), "i18n".to_string()])
    .build()?;

    node.register_mcp_tool(greeting_tool).await?;
    println!("Registered greeting tool");

    // List all registered tools
    let tools = node.list_mcp_tools().await?;
    println!("Available tools: {:?}", tools);

    println!();
    println!("Demonstrating tool calls:");
    println!("=========================");

    // Test calculator tool
    println!("1. Calculator: 15 + 27");
    let result = node.call_mcp_tool("calculator", json!({
        "a": 15,
        "b": 27,
        "operation": "add"
    })).await?;
    println!("   Result: {}", result["result"]);

    println!("2. Calculator: 144 / 12");
    let result = node.call_mcp_tool("calculator", json!({
        "a": 144,
        "b": 12,
        "operation": "divide"
    })).await?;
    println!("   Result: {}", result["result"]);

    // Test timestamp tool
    println!("3. Current timestamp");
    let result = node.call_mcp_tool("timestamp", json!({})).await?;
    println!("   Timestamp: {}", result["timestamp"]);
    println!("   Formatted: {}", result["formatted"]);

    // Test greeting tool
    println!("4. Greeting in Spanish");
    let result = node.call_mcp_tool("greeting", json!({
        "name": "María",
        "language": "spanish"
    })).await?;
    println!("   Greeting: {}", result["greeting"]);

    println!("5. Greeting in Japanese");
    let result = node.call_mcp_tool("greeting", json!({
        "name": "田中",
        "language": "japanese"
    })).await?;
    println!("   Greeting: {}", result["greeting"]);

    // Test error handling
    println!("6. Testing error handling (division by zero)");
    match node.call_mcp_tool("calculator", json!({
        "a": 10,
        "b": 0,
        "operation": "divide"
    })).await {
        Ok(result) => println!("   Unexpected success: {}", result),
        Err(e) => println!("   Expected error: {}", e),
    }

    // Show MCP server statistics
    println!();
    println!("MCP Server Statistics:");
    println!("=====================");
    let stats = node.mcp_stats().await?;
    println!("Total requests: {}", stats.total_requests);
    println!("Total responses: {}", stats.total_responses);
    println!("Total tools: {}", stats.total_tools);
    println!("Average response time: {:?}", stats.avg_response_time);
    println!("Popular tools: {:?}", stats.popular_tools);

    println!();
    println!("MCP server is running at: {}", node.peer_id());
    println!("Tools are registered and available for remote calls via P2P network.");
    println!();
    println!("Press Ctrl+C to stop the server...");

    // Keep the server running until interrupted
    tokio::signal::ctrl_c().await?;
    
    println!("Shutting down...");
    node.stop().await?;
    println!("MCP service example completed successfully!");

    Ok(())
}