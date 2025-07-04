
//! MCP (Model Context Protocol) server integration tests
//!
//! Comprehensive tests for MCP server functionality including:
//! - Tool registration and discovery
//! - Remote procedure calls
//! - Service capability advertisement
//! - Cross-node MCP communication
//! - Performance and reliability

use anyhow::Result;
use std::time::Duration;
use std::collections::HashMap;
use serde_json::{json, Value};
use tokio::time::timeout;

use p2p_foundation::{P2PNode, mcp::*};
use crate::common::{TestNetwork, PerformanceTest};

// Integration test submodules - TBD
// mod tools;
// mod services;
// mod discovery;
// mod rpc;
// mod streaming;

/// Test basic MCP server setup and tool registration
#[tokio::test]
async fn test_mcp_server_basic_setup() -> Result<()> {
    let config = TestNodeConfig::builder()
        .port(9300)
        .enable_mcp(true)
        .build();
    
    let node = P2PNode::new(config).await?;
    
    // Verify MCP server is running
    let mcp_server = node.mcp_server().await?;
    assert!(mcp_server.is_running());
    
    // Register a basic tool
    let calculator_tool = Tool::new("calculator")
        .description("Basic calculator operations")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "operation": {"type": "string", "enum": ["add", "subtract", "multiply", "divide"]},
                "a": {"type": "number"},
                "b": {"type": "number"}
            },
            "required": ["operation", "a", "b"]
        }))
        .handler(Box::new(|params: Value| {
            Box::pin(async move {
                let operation = params["operation"].as_str().unwrap();
                let a = params["a"].as_f64().unwrap();
                let b = params["b"].as_f64().unwrap();
                
                let result = match operation {
                    "add" => a + b,
                    "subtract" => a - b,
                    "multiply" => a * b,
                    "divide" => {
                        if b != 0.0 { a / b } else { return Err(anyhow::anyhow!("Division by zero")); }
                    },
                    _ => return Err(anyhow::anyhow!("Unknown operation")),
                };
                
                Ok(json!({"result": result}))
            })
        }));
    
    mcp_server.register_tool(calculator_tool).await?;
    
    // Verify tool is registered
    let tools = mcp_server.list_tools().await?;
    assert!(tools.iter().any(|tool| tool.name == "calculator"));
    
    // Test tool execution
    let params = json!({
        "operation": "add",
        "a": 5.0,
        "b": 3.0
    });
    
    let result = mcp_server.call_tool("calculator", params).await?;
    assert_eq!(result["result"], json!(8.0));
    
    node.stop().await?;
    Ok(())
}

/// Test MCP service discovery across nodes
#[tokio::test]
async fn test_mcp_service_discovery() -> Result<()> {
    let network = TestNetwork::simple(3).await?;
    
    // Register different tools on each node
    let node1_tools = vec![
        create_file_operations_tool(),
        create_text_processing_tool(),
    ];
    
    let node2_tools = vec![
        create_network_operations_tool(),
        create_crypto_operations_tool(),
    ];
    
    let node3_tools = vec![
        create_data_analysis_tool(),
        create_ai_operations_tool(),
    ];
    
    // Register tools on respective nodes
    for tool in node1_tools {
        network.node(0)?.mcp_server().await?.register_tool(tool).await?;
    }
    
    for tool in node2_tools {
        network.node(1)?.mcp_server().await?.register_tool(tool).await?;
    }
    
    for tool in node3_tools {
        network.node(2)?.mcp_server().await?.register_tool(tool).await?;
    }
    
    // Wait for service discovery propagation
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    // Each node should discover services from other nodes
    for i in 0..3 {
        let discovered_services = network.node(i)?.mcp_discover_services().await?;
        
        // Should discover services from other nodes
        assert!(
            discovered_services.len() >= 4, // At least 4 tools from other nodes
            "Node {} should discover services from other nodes, found {}",
            i, discovered_services.len()
        );
        
        // Verify we can see capabilities from remote nodes
        let remote_capabilities = network.node(i)?.mcp_get_remote_capabilities().await?;
        assert!(remote_capabilities.len() >= 2); // At least 2 other nodes
        
        for (peer_id, capabilities) in remote_capabilities {
            assert!(!capabilities.tools.is_empty());
            println!("Node {} discovered {} tools from peer {}", 
                    i, capabilities.tools.len(), peer_id);
        }
    }
    
    network.stop().await?;
    Ok(())
}

/// Test remote MCP tool invocation
#[tokio::test]
async fn test_remote_mcp_tool_invocation() -> Result<()> {
    let network = TestNetwork::simple(2).await?;
    
    // Register calculator tool on node 1
    let calculator_tool = create_calculator_tool();
    network.node(1)?.mcp_server().await?.register_tool(calculator_tool).await?;
    
    // Wait for service discovery
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Call remote tool from node 0
    let node1_id = network.node(1)?.peer_id();
    let params = json!({
        "operation": "multiply",
        "a": 7.0,
        "b": 6.0
    });
    
    let start = std::time::Instant::now();
    let result = network.node(0)?.mcp_call_remote_tool(
        &node1_id,
        "calculator",
        params
    ).await?;
    let call_duration = start.elapsed();
    
    println!("Remote MCP call took: {:?}", call_duration);
    
    assert_eq!(result["result"], json!(42.0));
    
    // Test error handling for non-existent tool
    let error_result = network.node(0)?.mcp_call_remote_tool(
        &node1_id,
        "nonexistent_tool",
        json!({})
    ).await;
    
    assert!(error_result.is_err());
    
    // Test error handling for invalid parameters
    let invalid_params = json!({
        "operation": "divide",
        "a": "not_a_number",
        "b": 5.0
    });
    
    let param_error = network.node(0)?.mcp_call_remote_tool(
        &node1_id,
        "calculator", 
        invalid_params
    ).await;
    
    assert!(param_error.is_err());
    
    network.stop().await?;
    Ok(())
}

/// Test MCP service with stateful operations
#[tokio::test]
async fn test_mcp_stateful_service() -> Result<()> {
    let network = TestNetwork::simple(2).await?;
    
    // Create stateful key-value store service
    let kv_store_tool = Tool::new("kv_store")
        .description("Distributed key-value store operations")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "action": {"type": "string", "enum": ["get", "set", "delete", "list"]},
                "key": {"type": "string"},
                "value": {"type": "string"}
            },
            "required": ["action"]
        }))
        .stateful(true)
        .handler(Box::new(|params: Value| {
            Box::pin(async move {
                // In real implementation, this would use a persistent store
                static mut STORE: Option<HashMap<String, String>> = None;
                
                unsafe {
                    if STORE.is_none() {
                        STORE = Some(HashMap::new());
                    }
                    
                    let store = STORE.as_mut().unwrap();
                    let action = params["action"].as_str().unwrap();
                    
                    match action {
                        "set" => {
                            let key = params["key"].as_str().unwrap().to_string();
                            let value = params["value"].as_str().unwrap().to_string();
                            store.insert(key.clone(), value.clone());
                            Ok(json!({"status": "stored", "key": key, "value": value}))
                        },
                        "get" => {
                            let key = params["key"].as_str().unwrap();
                            match store.get(key) {
                                Some(value) => Ok(json!({"key": key, "value": value})),
                                None => Ok(json!({"key": key, "value": null}))
                            }
                        },
                        "delete" => {
                            let key = params["key"].as_str().unwrap();
                            let removed = store.remove(key);
                            Ok(json!({"status": "deleted", "key": key, "existed": removed.is_some()}))
                        },
                        "list" => {
                            let keys: Vec<&String> = store.keys().collect();
                            Ok(json!({"keys": keys}))
                        },
                        _ => Err(anyhow::anyhow!("Unknown action")),
                    }
                }
            })
        }));
    
    network.node(1)?.mcp_server().await?.register_tool(kv_store_tool).await?;
    
    // Wait for service discovery
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    let node1_id = network.node(1)?.peer_id();
    
    // Test stateful operations
    // Set a value
    let set_result = network.node(0)?.mcp_call_remote_tool(
        &node1_id,
        "kv_store",
        json!({"action": "set", "key": "test_key", "value": "test_value"})
    ).await?;
    
    assert_eq!(set_result["status"], "stored");
    
    // Get the value
    let get_result = network.node(0)?.mcp_call_remote_tool(
        &node1_id,
        "kv_store",
        json!({"action": "get", "key": "test_key"})
    ).await?;
    
    assert_eq!(get_result["value"], "test_value");
    
    // List keys
    let list_result = network.node(0)?.mcp_call_remote_tool(
        &node1_id,
        "kv_store",
        json!({"action": "list"})
    ).await?;
    
    assert!(list_result["keys"].as_array().unwrap().contains(&json!("test_key")));
    
    // Delete the value
    let delete_result = network.node(0)?.mcp_call_remote_tool(
        &node1_id,
        "kv_store",
        json!({"action": "delete", "key": "test_key"})
    ).await?;
    
    assert_eq!(delete_result["existed"], true);
    
    // Verify deletion
    let get_deleted = network.node(0)?.mcp_call_remote_tool(
        &node1_id,
        "kv_store",
        json!({"action": "get", "key": "test_key"})
    ).await?;
    
    assert_eq!(get_deleted["value"], json!(null));
    
    network.stop().await?;
    Ok(())
}

/// Test MCP streaming operations
#[tokio::test]
async fn test_mcp_streaming() -> Result<()> {
    let network = TestNetwork::simple(2).await?;
    
    // Create streaming tool that generates a sequence of numbers
    let number_stream_tool = Tool::new("number_stream")
        .description("Generate a stream of numbers")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "start": {"type": "number"},
                "end": {"type": "number"},
                "step": {"type": "number"}
            },
            "required": ["start", "end"]
        }))
        .streaming(true)
        .handler(Box::new(|params: Value| {
            Box::pin(async move {
                let start = params["start"].as_f64().unwrap();
                let end = params["end"].as_f64().unwrap();
                let step = params["step"].as_f64().unwrap_or(1.0);
                
                let mut numbers = Vec::new();
                let mut current = start;
                
                while current <= end {
                    numbers.push(current);
                    current += step;
                }
                
                Ok(json!({"numbers": numbers, "streaming": true}))
            })
        }));
    
    network.node(1)?.mcp_server().await?.register_tool(number_stream_tool).await?;
    
    // Wait for service discovery
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    let node1_id = network.node(1)?.peer_id();
    
    // Test streaming call
    let params = json!({"start": 1.0, "end": 10.0, "step": 2.0});
    
    let mut stream = network.node(0)?.mcp_call_remote_tool_streaming(
        &node1_id,
        "number_stream",
        params
    ).await?;
    
    let mut received_numbers = Vec::new();
    
    // Collect streaming results
    while let Some(chunk) = timeout(Duration::from_secs(5), stream.next()).await? {
        let chunk = chunk?;
        if let Some(numbers) = chunk["numbers"].as_array() {
            for number in numbers {
                received_numbers.push(number.as_f64().unwrap());
            }
        }
    }
    
    assert_eq!(received_numbers, vec![1.0, 3.0, 5.0, 7.0, 9.0]);
    
    network.stop().await?;
    Ok(())
}

/// Test MCP service load balancing
#[tokio::test]
async fn test_mcp_load_balancing() -> Result<()> {
    let network = TestNetwork::simple(4).await?;
    
    // Register the same service on multiple nodes
    let echo_tool = create_echo_tool();
    
    for i in 1..4 { // Nodes 1, 2, 3
        network.node(i)?.mcp_server().await?.register_tool(echo_tool.clone()).await?;
    }
    
    // Wait for service discovery
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    // Make many calls from node 0 and track which nodes handle them
    let mut node_call_counts: HashMap<String, usize> = HashMap::new();
    let total_calls = 30;
    
    for i in 0..total_calls {
        let params = json!({"message": format!("test_message_{}", i)});
        
        let (result, handling_peer) = network.node(0)?.mcp_call_with_load_balancing(
            "echo",
            params
        ).await?;
        
        assert_eq!(result["echo"], format!("test_message_{}", i));
        
        *node_call_counts.entry(handling_peer.to_string()).or_insert(0) += 1;
    }
    
    println!("Call distribution: {:?}", node_call_counts);
    
    // Verify load balancing - calls should be distributed across nodes
    assert!(node_call_counts.len() >= 2, "Calls should be distributed across multiple nodes");
    
    // No single node should handle more than 70% of calls
    for (node_id, count) in node_call_counts {
        let percentage = *count as f64 / total_calls as f64;
        assert!(
            percentage < 0.7,
            "Node {} handled too many calls: {:.1}%",
            node_id, percentage * 100.0
        );
    }
    
    network.stop().await?;
    Ok(())
}

/// Test MCP service authentication and authorization
#[tokio::test]
async fn test_mcp_auth() -> Result<()> {
    let config1 = TestNodeConfig::builder()
        .port(9310)
        .enable_mcp(true)
        .build();
    let mut auth_config1 = config1;
    auth_config1.mcp_require_auth = true;
    auth_config1.mcp_allowed_peers = vec!["trusted_peer".to_string()];
    
    let config2 = TestNodeConfig::builder()
        .port(9311)
        .enable_mcp(true)
        .build();
    
    let node1 = P2PNode::new(auth_config1).await?;
    let node2 = P2PNode::new(config2).await?;
    
    // Connect nodes
    let node1_addrs = node1.listen_addrs().await?;
    node2.connect(node1_addrs[0].clone()).await?;
    
    // Register protected tool on node1
    let protected_tool = Tool::new("protected_operation")
        .description("Operation requiring authentication")
        .input_schema(json!({"type": "object"}))
        .requires_auth(true)
        .handler(Box::new(|_params: Value| {
            Box::pin(async move {
                Ok(json!({"status": "authenticated_success"}))
            })
        }));
    
    node1.mcp_server().await?.register_tool(protected_tool).await?;
    
    // Wait for discovery
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    let node1_id = node1.peer_id();
    
    // Try to call without authentication - should fail
    let unauth_result = node2.mcp_call_remote_tool(
        &node1_id,
        "protected_operation",
        json!({})
    ).await;
    
    assert!(unauth_result.is_err());
    
    // Authenticate node2 with node1
    node2.mcp_authenticate(&node1_id, "trusted_peer", "secret_token").await?;
    
    // Now the call should succeed
    let auth_result = node2.mcp_call_remote_tool(
        &node1_id,
        "protected_operation",
        json!({})
    ).await?;
    
    assert_eq!(auth_result["status"], "authenticated_success");
    
    node1.stop().await?;
    node2.stop().await?;
    Ok(())
}

/// Performance test for MCP operations
#[tokio::test]
async fn test_mcp_performance() -> Result<()> {
    let network = TestNetwork::simple(2).await?;
    let mut perf = PerformanceTest::new();
    
    // Register performance test tool
    let perf_tool = Tool::new("perf_test")
        .description("Performance testing tool")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "data_size": {"type": "number"},
                "operation": {"type": "string"}
            }
        }))
        .handler(Box::new(|params: Value| {
            Box::pin(async move {
                let data_size = params["data_size"].as_u64().unwrap_or(1024);
                let operation = params["operation"].as_str().unwrap_or("echo");
                
                let data = vec![0u8; data_size as usize];
                
                match operation {
                    "echo" => Ok(json!({"result": "echo", "size": data_size})),
                    "process" => {
                        // Simulate some processing
                        let checksum: u32 = data.iter().map(|&b| b as u32).sum();
                        Ok(json!({"result": "processed", "checksum": checksum, "size": data_size}))
                    },
                    _ => Err(anyhow::anyhow!("Unknown operation")),
                }
            })
        }));
    
    network.node(1)?.mcp_server().await?.register_tool(perf_tool).await?;
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    let node1_id = network.node(1)?.peer_id();
    
    // Test small message performance
    let small_msg_time = perf.measure_async("small_messages", async {
        for _ in 0..100 {
            network.node(0)?.mcp_call_remote_tool(
                &node1_id,
                "perf_test",
                json!({"data_size": 100, "operation": "echo"})
            ).await?;
        }
        Ok::<(), anyhow::Error>(())
    }).await?;
    
    // Test large message performance
    let large_msg_time = perf.measure_async("large_messages", async {
        for _ in 0..10 {
            network.node(0)?.mcp_call_remote_tool(
                &node1_id,
                "perf_test",
                json!({"data_size": 10240, "operation": "process"})
            ).await?;
        }
        Ok::<(), anyhow::Error>(())
    }).await?;
    
    // Test concurrent calls
    let concurrent_time = perf.measure_async("concurrent_calls", async {
        let mut handles = Vec::new();
        
        for i in 0..20 {
            let node = network.node(0)?;
            let node1_id = node1_id.clone();
            
            let handle = tokio::spawn(async move {
                node.mcp_call_remote_tool(
                    &node1_id,
                    "perf_test",
                    json!({"data_size": 512, "operation": "echo"})
                ).await
            });
            handles.push(handle);
        }
        
        for handle in handles {
            handle.await??;
        }
        
        Ok::<(), anyhow::Error>(())
    }).await?;
    
    perf.print_results();
    
    // Performance assertions
    let avg_small_time = small_msg_time / 100;
    let avg_large_time = large_msg_time / 10;
    let avg_concurrent_time = concurrent_time / 20;
    
    println!("Average small message time: {:?}", avg_small_time);
    println!("Average large message time: {:?}", avg_large_time);
    println!("Average concurrent call time: {:?}", avg_concurrent_time);
    
    assert!(avg_small_time < Duration::from_millis(50), "Small messages too slow");
    assert!(avg_large_time < Duration::from_millis(200), "Large messages too slow");
    assert!(concurrent_time < Duration::from_secs(5), "Concurrent calls too slow");
    
    network.stop().await?;
    Ok(())
}

// Helper functions for creating test tools

fn create_calculator_tool() -> Tool {
    Tool::new("calculator")
        .description("Basic calculator operations")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "operation": {"type": "string", "enum": ["add", "subtract", "multiply", "divide"]},
                "a": {"type": "number"},
                "b": {"type": "number"}
            },
            "required": ["operation", "a", "b"]
        }))
        .handler(Box::new(|params: Value| {
            Box::pin(async move {
                let operation = params["operation"].as_str().unwrap();
                let a = params["a"].as_f64().unwrap();
                let b = params["b"].as_f64().unwrap();
                
                let result = match operation {
                    "add" => a + b,
                    "subtract" => a - b,
                    "multiply" => a * b,
                    "divide" => {
                        if b != 0.0 { a / b } else { return Err(anyhow::anyhow!("Division by zero")); }
                    },
                    _ => return Err(anyhow::anyhow!("Unknown operation")),
                };
                
                Ok(json!({"result": result}))
            })
        }))
}

fn create_echo_tool() -> Tool {
    Tool::new("echo")
        .description("Echo back the input message")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "message": {"type": "string"}
            },
            "required": ["message"]
        }))
        .handler(Box::new(|params: Value| {
            Box::pin(async move {
                let message = params["message"].as_str().unwrap();
                Ok(json!({"echo": message}))
            })
        }))
}

fn create_file_operations_tool() -> Tool {
    Tool::new("file_ops")
        .description("File system operations")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "action": {"type": "string", "enum": ["read", "write", "list"]},
                "path": {"type": "string"}
            }
        }))
        .handler(Box::new(|_params: Value| {
            Box::pin(async move {
                Ok(json!({"status": "simulated"}))
            })
        }))
}

fn create_text_processing_tool() -> Tool {
    Tool::new("text_process")
        .description("Text processing operations")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "text": {"type": "string"},
                "operation": {"type": "string"}
            }
        }))
        .handler(Box::new(|_params: Value| {
            Box::pin(async move {
                Ok(json!({"result": "processed"}))
            })
        }))
}

fn create_network_operations_tool() -> Tool {
    Tool::new("network_ops")
        .description("Network operations")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "action": {"type": "string"},
                "target": {"type": "string"}
            }
        }))
        .handler(Box::new(|_params: Value| {
            Box::pin(async move {
                Ok(json!({"status": "completed"}))
            })
        }))
}

fn create_crypto_operations_tool() -> Tool {
    Tool::new("crypto_ops")
        .description("Cryptographic operations")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "operation": {"type": "string"},
                "data": {"type": "string"}
            }
        }))
        .handler(Box::new(|_params: Value| {
            Box::pin(async move {
                Ok(json!({"result": "hashed"}))
            })
        }))
}

fn create_data_analysis_tool() -> Tool {
    Tool::new("data_analysis")
        .description("Data analysis operations")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "dataset": {"type": "array"},
                "analysis_type": {"type": "string"}
            }
        }))
        .handler(Box::new(|_params: Value| {
            Box::pin(async move {
                Ok(json!({"analysis": "completed"}))
            })
        }))
}

fn create_ai_operations_tool() -> Tool {
    Tool::new("ai_ops")
        .description("AI model operations")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "model": {"type": "string"},
                "input": {"type": "string"}
            }
        }))
        .handler(Box::new(|_params: Value| {
            Box::pin(async move {
                Ok(json!({"prediction": "result"}))
            })
        }))
}