
// Quick API test to verify imports and types
use saorsa_core::{P2PNode, NodeBuilder, P2PEvent, Key};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Test NodeBuilder
    let node = NodeBuilder::new()
        .with_dht()
        .with_mcp_server()
        .with_production_config()
        .build()
        .await?;
    
    // Test methods
    let _peer_id = node.peer_id();
    let _addrs = node.listen_addrs().await;
    let _events = node.subscribe_events();
    let _peers = node.connected_peers().await;
    
    // Test DHT
    let key = Key::new(b"test");
    let _put = node.dht_put(key.clone(), vec![1, 2, 3]).await?;
    let _get = node.dht_get(key).await?;
    
    // Test send_message
    node.send_message(&"peer123".to_string(), "/test/1.0.0", vec![]).await?;
    
    // Test connect_peer
    let _peer = node.connect_peer("/ip4/127.0.0.1/tcp/9000").await?;
    
    // Test shutdown
    node.shutdown().await?;
    
    Ok(())
}