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

//! MCP Service Discovery Demo
//!
//! This example demonstrates the automatic MCP service announcement and discovery
//! functionality in the P2P Foundation. It creates multiple nodes with different
//! tools and shows how they discover each other's services automatically.

use p2p_foundation::{P2PNode, NodeConfig, Result};
use p2p_foundation::mcp::{Tool, FunctionToolHandler, MCPServerConfig};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::init();
    
    println!("🚀 Starting MCP Service Discovery Demo");
    
    // Create three nodes with different specialized tools
    let calculator_node = create_calculator_node().await?;
    let weather_node = create_weather_node().await?;
    let translator_node = create_translator_node().await?;
    
    println!("📡 Starting P2P nodes...");
    
    // Start all nodes
    calculator_node.start().await?;
    weather_node.start().await?;
    translator_node.start().await?;
    
    sleep(Duration::from_millis(500)).await;
    
    // Connect nodes to form a network
    println!("🔗 Connecting nodes to form P2P network...");
    
    calculator_node.connect_peer("/ip4/127.0.0.1/tcp/9001").await.ok();
    calculator_node.connect_peer("/ip4/127.0.0.1/tcp/9002").await.ok();
    weather_node.connect_peer("/ip4/127.0.0.1/tcp/9002").await.ok();
    
    sleep(Duration::from_millis(1000)).await;
    
    // Announce services
    println!("📢 Announcing local services...");
    
    calculator_node.announce_mcp_services().await?;
    weather_node.announce_mcp_services().await?;
    translator_node.announce_mcp_services().await?;
    
    sleep(Duration::from_millis(500)).await;
    
    // Trigger service discovery
    println!("🔍 Broadcasting service discovery queries...");
    
    calculator_node.broadcast_mcp_service_discovery().await?;
    weather_node.broadcast_mcp_service_discovery().await?;
    translator_node.broadcast_mcp_service_discovery().await?;
    
    sleep(Duration::from_millis(1000)).await;
    
    // Refresh service discovery to get latest information
    println!("🔄 Refreshing service discovery...");
    
    calculator_node.refresh_mcp_service_discovery().await?;
    weather_node.refresh_mcp_service_discovery().await?;
    translator_node.refresh_mcp_service_discovery().await?;
    
    sleep(Duration::from_millis(500)).await;
    
    // Display discovered services from each node's perspective
    println!("\n📋 Discovered Services Report:");
    println!("===============================");
    
    display_node_services("Calculator Node", &calculator_node).await?;
    display_node_services("Weather Node", &weather_node).await?;
    display_node_services("Translator Node", &translator_node).await?;
    
    // Demonstrate finding services by tool type
    println!("\n🎯 Finding Services by Tool Type:");
    println!("==================================");
    
    let calc_services = calculator_node.find_mcp_services_with_tool("add").await?;
    println!("Nodes with 'add' tool: {} found", calc_services.len());
    
    let weather_services = weather_node.find_mcp_services_with_tool("get_weather").await?;
    println!("Nodes with 'get_weather' tool: {} found", weather_services.len());
    
    let translate_services = translator_node.find_mcp_services_with_tool("translate").await?;
    println!("Nodes with 'translate' tool: {} found", translate_services.len());
    
    // Demonstrate cross-node tool calling
    println!("\n🔧 Testing Cross-Node Tool Calls:");
    println!("==================================");
    
    // Find a node that has the weather tool and call it from calculator node
    if let Some(weather_service) = weather_services.first() {
        println!("Calling weather tool from calculator node...");
        
        match calculator_node.call_remote_mcp_tool(
            &weather_service.node_id,
            "get_weather",
            json!({"location": "San Francisco"})
        ).await {
            Ok(result) => println!("Weather result: {}", result),
            Err(e) => println!("Weather call failed: {}", e),
        }
    }
    
    sleep(Duration::from_millis(1000)).await;
    
    // Graceful shutdown
    println!("\n🛑 Shutting down nodes...");
    
    calculator_node.stop().await?;
    weather_node.stop().await?;
    translator_node.stop().await?;
    
    println!("✅ MCP Service Discovery Demo completed successfully!");
    
    Ok(())
}

async fn create_calculator_node() -> Result<Arc<P2PNode>> {
    let mut mcp_config = MCPServerConfig::default();
    mcp_config.enable_auth = false;
    mcp_config.enable_dht_discovery = true;
    mcp_config.server_name = "calculator_service".to_string();
    
    let config = NodeConfig {
        peer_id: Some("calculator_node".to_string()),
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/9000".to_string()],
        enable_mcp_server: true,
        mcp_server_config: Some(mcp_config),
        ..NodeConfig::default()
    };
    
    let node = Arc::new(P2PNode::new(config).await?);
    
    // Register calculator tools
    let add_tool = Tool::new(
        "add",
        "Add two numbers",
        json!({
            "type": "object",
            "properties": {
                "a": {"type": "number"},
                "b": {"type": "number"}
            },
            "required": ["a", "b"]
        })
    ).handler(FunctionToolHandler::new(|args: Value| async move {
        let a = args["a"].as_f64().unwrap_or(0.0);
        let b = args["b"].as_f64().unwrap_or(0.0);
        Ok(json!({"result": a + b}))
    })).build()?;
    
    let multiply_tool = Tool::new(
        "multiply",
        "Multiply two numbers",
        json!({
            "type": "object",
            "properties": {
                "a": {"type": "number"},
                "b": {"type": "number"}
            },
            "required": ["a", "b"]
        })
    ).handler(FunctionToolHandler::new(|args: Value| async move {
        let a = args["a"].as_f64().unwrap_or(0.0);
        let b = args["b"].as_f64().unwrap_or(0.0);
        Ok(json!({"result": a * b}))
    })).build()?;
    
    node.register_mcp_tool(add_tool).await?;
    node.register_mcp_tool(multiply_tool).await?;
    
    Ok(node)
}

async fn create_weather_node() -> Result<Arc<P2PNode>> {
    let mut mcp_config = MCPServerConfig::default();
    mcp_config.enable_auth = false;
    mcp_config.enable_dht_discovery = true;
    mcp_config.server_name = "weather_service".to_string();
    
    let config = NodeConfig {
        peer_id: Some("weather_node".to_string()),
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/9001".to_string()],
        enable_mcp_server: true,
        mcp_server_config: Some(mcp_config),
        ..NodeConfig::default()
    };
    
    let node = Arc::new(P2PNode::new(config).await?);
    
    // Register weather tools
    let weather_tool = Tool::new(
        "get_weather",
        "Get weather information for a location",
        json!({
            "type": "object",
            "properties": {
                "location": {"type": "string", "description": "Location name"}
            },
            "required": ["location"]
        })
    ).handler(FunctionToolHandler::new(|args: Value| async move {
        let location = args["location"].as_str().unwrap_or("Unknown");
        Ok(json!({
            "location": location,
            "temperature": "22°C",
            "condition": "Sunny",
            "humidity": "65%"
        }))
    })).build()?;
    
    let forecast_tool = Tool::new(
        "get_forecast",
        "Get weather forecast",
        json!({
            "type": "object",
            "properties": {
                "location": {"type": "string"},
                "days": {"type": "number", "default": 5}
            },
            "required": ["location"]
        })
    ).handler(FunctionToolHandler::new(|args: Value| async move {
        let location = args["location"].as_str().unwrap_or("Unknown");
        let days = args["days"].as_i64().unwrap_or(5);
        Ok(json!({
            "location": location,
            "forecast": format!("{} day forecast for {}", days, location),
            "outlook": "Generally sunny with occasional clouds"
        }))
    })).build()?;
    
    node.register_mcp_tool(weather_tool).await?;
    node.register_mcp_tool(forecast_tool).await?;
    
    Ok(node)
}

async fn create_translator_node() -> Result<Arc<P2PNode>> {
    let mut mcp_config = MCPServerConfig::default();
    mcp_config.enable_auth = false;
    mcp_config.enable_dht_discovery = true;
    mcp_config.server_name = "translator_service".to_string();
    
    let config = NodeConfig {
        peer_id: Some("translator_node".to_string()),
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/9002".to_string()],
        enable_mcp_server: true,
        mcp_server_config: Some(mcp_config),
        ..NodeConfig::default()
    };
    
    let node = Arc::new(P2PNode::new(config).await?);
    
    // Register translation tools
    let translate_tool = Tool::new(
        "translate",
        "Translate text between languages",
        json!({
            "type": "object",
            "properties": {
                "text": {"type": "string"},
                "from": {"type": "string", "default": "auto"},
                "to": {"type": "string", "default": "en"}
            },
            "required": ["text", "to"]
        })
    ).handler(FunctionToolHandler::new(|args: Value| async move {
        let text = args["text"].as_str().unwrap_or("");
        let to_lang = args["to"].as_str().unwrap_or("en");
        Ok(json!({
            "original": text,
            "translated": format!("[{}] {}", to_lang.to_uppercase(), text),
            "confidence": 0.95
        }))
    })).build()?;
    
    let detect_language_tool = Tool::new(
        "detect_language",
        "Detect the language of text",
        json!({
            "type": "object",
            "properties": {
                "text": {"type": "string"}
            },
            "required": ["text"]
        })
    ).handler(FunctionToolHandler::new(|args: Value| async move {
        let _text = args["text"].as_str().unwrap_or("");
        Ok(json!({
            "language": "en",
            "confidence": 0.92,
            "alternatives": ["es", "fr"]
        }))
    })).build()?;
    
    node.register_mcp_tool(translate_tool).await?;
    node.register_mcp_tool(detect_language_tool).await?;
    
    Ok(node)
}

async fn display_node_services(node_name: &str, node: &Arc<P2PNode>) -> Result<()> {
    println!("\n{} Services:", node_name);
    println!("{}", "-".repeat(node_name.len() + 10));
    
    let services = node.get_all_mcp_services().await?;
    
    if services.is_empty() {
        println!("  No services discovered");
        return Ok(());
    }
    
    for service in services {
        println!("  📦 Service: {} (Node: {})", service.metadata.name, service.node_id);
        println!("     🏷️  Description: {}", service.metadata.description.as_deref().unwrap_or("No description"));
        println!("     🔧 Tools ({}):", service.tools.len());
        
        for tool in &service.tools {
            println!("        • {}", tool);
        }
        
        println!("     💚 Health: {:?}", service.metadata.health_status);
        println!("     🌐 Endpoint: {}://{}", service.endpoint.protocol, service.endpoint.address);
        println!();
    }
    
    Ok(())
}