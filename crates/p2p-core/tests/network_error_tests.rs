// Network module error handling tests

use p2p_core::error::{NetworkError, P2PError, Result};
use p2p_core::network::{P2PNode, P2PNodeConfig};
use std::net::SocketAddr;
use std::time::Duration;

#[tokio::test]
async fn test_invalid_address_parsing() {
    // Test that invalid addresses return proper errors instead of panicking
    let invalid_addrs = vec![
        "invalid:address",
        "256.256.256.256:8080",
        "localhost:not_a_port",
        "[invalid::ipv6]:8080",
    ];
    
    for addr in invalid_addrs {
        let result: Result<SocketAddr> = addr.parse()
            .map_err(|e| NetworkError::InvalidAddress {
                addr: addr.to_string(),
                reason: e.to_string(),
            }.into());
            
        assert!(result.is_err());
        if let Err(P2PError::Network(NetworkError::InvalidAddress { addr: a, .. })) = result {
            assert_eq!(a, addr);
        } else {
            panic!("Expected InvalidAddress error");
        }
    }
}

#[tokio::test]
async fn test_network_config_with_invalid_addresses() {
    // Test that config creation handles invalid addresses gracefully
    let mut config = P2PNodeConfig::default();
    
    // This should not panic
    let result = P2PNodeConfig::with_listen_addr("invalid:address");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_bind_error_handling() {
    // Test that binding to an invalid address returns proper error
    let mut config = P2PNodeConfig::default();
    
    // Try to bind to a privileged port (should fail without root)
    config.listen_addr = "127.0.0.1:80".parse().unwrap();
    
    let result = P2PNode::new(config).await;
    
    // Should get a bind error, not panic
    assert!(result.is_err());
    if let Err(P2PError::Network(NetworkError::BindError { addr, .. })) = result {
        assert_eq!(addr.port(), 80);
    }
}

#[tokio::test]
async fn test_connection_failure_handling() {
    // Test that connection failures return proper errors
    let config = P2PNodeConfig::default();
    let node = P2PNode::new(config).await.unwrap();
    
    // Try to connect to non-existent peer
    let result = node.connect_to_peer("192.168.255.255:9999").await;
    
    assert!(result.is_err());
    match result {
        Err(P2PError::Network(NetworkError::ConnectionFailed { peer, .. })) => {
            assert!(peer.contains("192.168.255.255"));
        }
        _ => panic!("Expected ConnectionFailed error"),
    }
}

#[tokio::test]
async fn test_peer_info_missing_handling() {
    // Test that missing peer info doesn't panic
    let config = P2PNodeConfig::default();
    let node = P2PNode::new(config).await.unwrap();
    
    // Request info for non-existent peer
    let result = node.get_peer_info("non_existent_peer_id");
    
    // Should return None or error, not panic
    assert!(result.is_none() || result.is_err());
}

#[tokio::test]
async fn test_event_stream_error_handling() {
    // Test that event stream errors don't panic
    let config = P2PNodeConfig::default();
    let mut node = P2PNode::new(config).await.unwrap();
    
    // Get event stream
    let mut events = node.events();
    
    // Shutdown node to cause stream to end
    node.shutdown().await;
    
    // Next event should be None or error, not panic
    let event = events.recv().await;
    assert!(event.is_none());
}

#[tokio::test]
async fn test_default_address_fallback() {
    // Test that default addresses are handled without unwrap
    let config = P2PNodeConfig::default();
    
    // Should have valid default addresses
    assert!(!config.bootstrap_peers.is_empty());
    
    // All default addresses should be valid
    for addr in &config.bootstrap_peers {
        let parsed: Result<SocketAddr> = addr.parse()
            .map_err(|e| NetworkError::InvalidAddress {
                addr: addr.to_string(),  
                reason: e.to_string(),
            }.into());
        assert!(parsed.is_ok());
    }
}

#[tokio::test]
async fn test_mcp_config_optional_handling() {
    // Test that missing MCP config doesn't panic
    let mut config = P2PNodeConfig::default();
    config.mcp_server_config = None;
    
    // Should create node without MCP, not panic
    let result = P2PNode::new(config).await;
    assert!(result.is_ok());
    
    let node = result.unwrap();
    // MCP operations should fail gracefully
    assert!(node.get_mcp_server().is_none());
}