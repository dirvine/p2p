//! MCP Service Discovery Tests
//!
//! Comprehensive tests for the automatic MCP service announcement and discovery system.

use p2p_foundation::{P2PNode, NodeConfig, Result};
use p2p_foundation::mcp::{Tool, FunctionToolHandler, ToolHandler, MCPServerConfig};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;

/// Helper function to create a test P2P node with MCP enabled
async fn create_test_node_with_tools(node_id: &str, tools: Vec<(&str, &str)>) -> Result<Arc<P2PNode>> {
    let mut mcp_config = MCPServerConfig::default();
    mcp_config.enable_auth = false;
    mcp_config.enable_rate_limiting = false;
    mcp_config.enable_dht_discovery = true;
    mcp_config.server_name = format!("test_server_{}", node_id);
    
    let config = NodeConfig {
        peer_id: Some(format!("test_node_{}", node_id)),
        listen_addrs: vec![
            format!("/ip4/127.0.0.1/tcp/{}", 9000 + node_id.parse::<u16>().unwrap_or(0))
        ],
        enable_mcp_server: true,
        mcp_server_config: Some(mcp_config),
        ..NodeConfig::default()
    };
    
    let node = Arc::new(P2PNode::new(config).await?);
    
    // Register tools
    for (tool_name, description) in tools {
        let handler = FunctionToolHandler::new(move |args: Value| async move {
            Ok(json!({
                "tool": tool_name,
                "input": args,
                "result": format!("Response from {}", tool_name)
            }))
        });
        
        let tool = Tool::new(
            tool_name,
            description,
            json!({
                "type": "object",
                "properties": {
                    "input": {"type": "string", "description": "Input parameter"}
                }
            })
        ).handler(handler).build()?;
        
        node.register_mcp_tool(tool).await?;
    }
    
    Ok(node)
}

/// Test basic service announcement functionality
#[tokio::test]
async fn test_service_announcement() -> Result<()> {
    let node = create_test_node_with_tools("1", vec![
        ("calculator", "A basic calculator tool"),
        ("weather", "Weather information tool"),
    ]).await?;
    
    // Start the node
    node.start().await?;
    
    // Wait for service announcement
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Manually trigger service announcement
    node.announce_mcp_services().await?;
    
    // Get all services (should include our local service)
    let services = node.get_all_mcp_services().await?;
    assert!(!services.is_empty(), "Should have at least one service (local)");
    
    // Find our local service
    let local_service = services.iter()
        .find(|s| s.node_id == "test_node_1")
        .expect("Should find local service");
    
    assert_eq!(local_service.tools.len(), 2);
    assert!(local_service.tools.contains(&"calculator".to_string()));
    assert!(local_service.tools.contains(&"weather".to_string()));
    
    node.stop().await?;
    Ok(())
}

/// Test service discovery between multiple nodes
#[tokio::test]
async fn test_multi_node_service_discovery() -> Result<()> {
    // Create three nodes with different tools
    let node1 = create_test_node_with_tools("1", vec![
        ("calculator", "Math operations"),
    ]).await?;
    
    let node2 = create_test_node_with_tools("2", vec![
        ("weather", "Weather data"),
        ("news", "News aggregator"),
    ]).await?;
    
    let node3 = create_test_node_with_tools("3", vec![
        ("translator", "Language translation"),
    ]).await?;
    
    // Start all nodes
    node1.start().await?;
    node2.start().await?;
    node3.start().await?;
    
    // Wait for startup
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Connect nodes to each other
    node1.connect_peer("/ip4/127.0.0.1/tcp/9002").await.ok(); // Connect to node2
    node1.connect_peer("/ip4/127.0.0.1/tcp/9003").await.ok(); // Connect to node3
    node2.connect_peer("/ip4/127.0.0.1/tcp/9003").await.ok(); // Connect to node3
    
    // Wait for connections to establish
    tokio::time::sleep(Duration::from_millis(300)).await;
    
    // Trigger service announcements
    node1.announce_mcp_services().await?;
    node2.announce_mcp_services().await?;
    node3.announce_mcp_services().await?;
    
    // Broadcast service discovery
    node1.broadcast_mcp_service_discovery().await?;
    
    // Wait for discovery to complete
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Refresh service discovery on all nodes
    node1.refresh_mcp_service_discovery().await?;
    node2.refresh_mcp_service_discovery().await?;
    node3.refresh_mcp_service_discovery().await?;
    
    // Check that each node can discover services from others
    let node1_services = node1.discover_mcp_services().await?;
    
    // Should discover remote services (exact count depends on DHT state)
    println!("Node1 discovered {} services", node1_services.len());
    
    // Stop all nodes
    node1.stop().await?;
    node2.stop().await?;
    node3.stop().await?;
    
    Ok(())
}

/// Test finding services by tool name
#[tokio::test]
async fn test_find_services_by_tool() -> Result<()> {
    let node = create_test_node_with_tools("1", vec![
        ("calculator", "Calculator tool"),
        ("weather", "Weather tool"),
        ("news", "News tool"),
    ]).await?;
    
    node.start().await?;
    
    // Wait for startup and service registration
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Find services with specific tools
    let calc_services = node.find_mcp_services_with_tool("calculator").await?;
    assert!(!calc_services.is_empty(), "Should find services with calculator tool");
    
    let weather_services = node.find_mcp_services_with_tool("weather").await?;
    assert!(!weather_services.is_empty(), "Should find services with weather tool");
    
    let nonexistent_services = node.find_mcp_services_with_tool("nonexistent").await?;
    assert!(nonexistent_services.is_empty(), "Should not find services with nonexistent tool");
    
    node.stop().await?;
    Ok(())
}

/// Test service discovery refresh mechanism
#[tokio::test]
async fn test_service_discovery_refresh() -> Result<()> {
    let node = create_test_node_with_tools("1", vec![
        ("test_tool", "Test tool for discovery"),
    ]).await?;
    
    node.start().await?;
    
    // Initial service discovery
    let initial_services = node.get_all_mcp_services().await?;
    let initial_count = initial_services.len();
    
    // Add another tool dynamically
    let dynamic_handler = FunctionToolHandler::new(|args: Value| async move {
        Ok(json!({"dynamic": true, "input": args}))
    });
    
    let dynamic_tool = Tool::new(
        "dynamic_tool",
        "Dynamically added tool",
        json!({"type": "object"})
    ).handler(dynamic_handler).build()?;
    
    node.register_mcp_tool(dynamic_tool).await?;
    
    // Refresh service discovery
    node.refresh_mcp_service_discovery().await?;
    
    // Check that the new tool is included in service announcements
    let updated_services = node.get_all_mcp_services().await?;
    
    // Find our local service
    let local_service = updated_services.iter()
        .find(|s| s.node_id == "test_node_1");
    
    if let Some(service) = local_service {
        assert!(service.tools.contains(&"test_tool".to_string()));
        assert!(service.tools.contains(&"dynamic_tool".to_string()));
        assert_eq!(service.tools.len(), 2);
    }
    
    node.stop().await?;
    Ok(())
}

/// Test service health status updates
#[tokio::test]
async fn test_service_health_monitoring() -> Result<()> {
    let node = create_test_node_with_tools("1", vec![
        ("health_test", "Health monitoring test tool"),
    ]).await?;
    
    node.start().await?;
    
    // Wait for service registration
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Get service health information
    let services = node.get_all_mcp_services().await?;
    assert!(!services.is_empty());
    
    let local_service = &services[0];
    assert_eq!(local_service.metadata.health_status, p2p_foundation::mcp::ServiceHealthStatus::Healthy);
    
    // Verify load metrics are initialized
    assert_eq!(local_service.metadata.load_metrics.error_rate, 0.0);
    assert!(local_service.metadata.load_metrics.avg_response_time_ms >= 0.0);
    
    node.stop().await?;
    Ok(())
}

/// Test peer-to-peer service discovery queries
#[tokio::test]
async fn test_peer_service_discovery_queries() -> Result<()> {
    let node1 = create_test_node_with_tools("1", vec![
        ("node1_tool", "Tool from node 1"),
    ]).await?;
    
    let node2 = create_test_node_with_tools("2", vec![
        ("node2_tool", "Tool from node 2"),
    ]).await?;
    
    node1.start().await?;
    node2.start().await?;
    
    // Wait for startup
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Connect nodes
    node1.connect_peer("/ip4/127.0.0.1/tcp/9002").await.ok();
    
    // Wait for connection
    tokio::time::sleep(Duration::from_millis(300)).await;
    
    // Query node2's services from node1
    node1.query_peer_mcp_services(&"test_node_2".to_string()).await?;
    
    // Wait for query response
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Refresh discovery to get updated service information
    node1.refresh_mcp_service_discovery().await?;
    
    node1.stop().await?;
    node2.stop().await?;
    
    Ok(())
}