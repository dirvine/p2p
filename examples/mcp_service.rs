//! MCP Service Example
//!
//! This example demonstrates how to create and use MCP services across nodes.

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    println!("MCP Service Example");
    println!("==================");
    println!();
    println!("This example will demonstrate MCP service functionality once the");
    println!("P2P Foundation library is implemented.");
    println!();
    println!("Expected functionality:");
    println!("- Create P2P nodes with MCP servers");
    println!("- Register tools and services");
    println!("- Discover services across the network");
    println!("- Call remote services");
    println!("- Handle service responses and errors");
    println!();
    println!("Implementation placeholder - library not yet available");

    // Placeholder for actual implementation:
    //
    // use p2p_foundation::{P2PNode, NodeConfig, Tool};
    // use serde_json::json;
    //
    // // Create nodes with MCP enabled
    // let config1 = NodeConfig {
    //     listen_addrs: vec!["/ip4/127.0.0.1/tcp/9000".parse()?],
    //     enable_mcp_server: true,
    //     ..Default::default()
    // };
    // let node1 = P2PNode::new(config1).await?;
    //
    // let config2 = NodeConfig {
    //     listen_addrs: vec!["/ip4/127.0.0.1/tcp/9001".parse()?],
    //     enable_mcp_server: true,
    //     ..Default::default()
    // };
    // let node2 = P2PNode::new(config2).await?;
    //
    // // Connect nodes
    // let addr1 = node1.listen_addrs().await?[0].clone();
    // node2.connect(addr1).await?;
    //
    // // Register a calculator tool on node1
    // let calculator = Tool::new("calculator")
    //     .description("Basic calculator")
    //     .input_schema(json!({
    //         "type": "object",
    //         "properties": {
    //             "a": {"type": "number"},
    //             "b": {"type": "number"},
    //             "op": {"type": "string"}
    //         }
    //     }))
    //     .handler(Box::new(|params| {
    //         Box::pin(async move {
    //             let a = params["a"].as_f64().unwrap();
    //             let b = params["b"].as_f64().unwrap();
    //             let op = params["op"].as_str().unwrap();
    //             
    //             let result = match op {
    //                 "add" => a + b,
    //                 "sub" => a - b,
    //                 "mul" => a * b,
    //                 "div" => a / b,
    //                 _ => return Err(anyhow::anyhow!("Unknown operation")),
    //             };
    //             
    //             Ok(json!({"result": result}))
    //         })
    //     }));
    //
    // node1.mcp_server().await?.register_tool(calculator).await?;
    //
    // // Call remote service from node2
    // let result = node2.mcp_call_remote_tool(
    //     &node1.peer_id(),
    //     "calculator",
    //     json!({"a": 5, "b": 3, "op": "add"})
    // ).await?;
    //
    // println!("Calculator result: {}", result);

    Ok(())
}