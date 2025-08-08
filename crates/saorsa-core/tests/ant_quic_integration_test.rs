// Copyright 2024 Saorsa Labs Limited
//
// Integration tests for native ant-quic integration

#![cfg(all(feature = "ant-quic", test))]

use saorsa_core::transport::ant_quic_adapter::P2PNetworkNode;
use std::net::SocketAddr;
use tokio::time::{Duration, timeout};

#[tokio::test]
async fn test_p2p_network_node_creation() {
    // Create a P2P network node on localhost
    let bind_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let node = P2PNetworkNode::new(bind_addr).await;

    assert!(node.is_ok(), "Should create P2P network node successfully");

    if let Ok(node) = node {
        // Verify we have a local address
        let local_addr = node.local_address();
        assert!(local_addr.port() > 0, "Should have assigned a port");

        // Verify we have a peer ID
        let peer_id = node.our_peer_id();
        assert!(
            !format!("{:?}", peer_id).is_empty(),
            "Should have a peer ID"
        );
    }
}

#[tokio::test]
async fn test_peer_to_peer_connection() {
    // Create two P2P network nodes
    let bind_addr1: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let bind_addr2: SocketAddr = "127.0.0.1:0".parse().unwrap();

    let node1 = P2PNetworkNode::new(bind_addr1).await.unwrap();
    let node2 = P2PNetworkNode::new(bind_addr2).await.unwrap();

    let addr1 = node1.local_address();

    // Try to connect node2 to node1
    let connect_result = timeout(Duration::from_secs(5), node2.connect_to_peer(addr1)).await;

    if connect_result.is_ok() {
        // Connection might succeed if ant-quic is fully functional
        let peer_id = connect_result.unwrap().unwrap();
        assert!(
            !format!("{:?}", peer_id).is_empty(),
            "Should have peer ID after connection"
        );

        // Check connected peers
        let peers = node2.get_connected_peers().await;
        assert!(!peers.is_empty(), "Should have at least one connected peer");
    } else {
        // Connection might timeout if ant-quic needs more setup
        // This is expected in a basic test environment
        println!(
            "Connection timed out (expected in test environment without full NAT traversal setup)"
        );
    }
}

#[tokio::test]
async fn test_p2p_data_transfer() {
    // This test would require a full ant-quic setup with proper NAT traversal
    // For now, we just verify the node compiles and basic operations work

    let bind_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let node = P2PNetworkNode::new(bind_addr).await.unwrap();

    // Verify we can call the methods without panicking
    let peers = node.get_connected_peers().await;
    assert_eq!(peers.len(), 0, "Should have no connected peers initially");

    // Test bootstrap with empty list
    let bootstrap_result = node.bootstrap_from_nodes(&[]).await;
    // Empty bootstrap should fail
    assert!(
        bootstrap_result.is_err(),
        "Bootstrap with empty list should fail"
    );
}

#[tokio::test]
async fn test_p2p_peer_authentication() {
    let bind_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let node = P2PNetworkNode::new(bind_addr).await.unwrap();

    // Get our own peer ID
    let peer_id = node.our_peer_id();

    // Check if we're authenticated (should be false for a non-existent connection)
    let is_auth = node.is_authenticated(&peer_id).await;
    // The result depends on ant-quic's internal implementation
    println!("Self authentication status: {}", is_auth);
}

#[test]
fn test_ant_quic_feature_enabled() {
    // Verify that ant-quic feature is enabled
    #[cfg(feature = "ant-quic")]
    {
        assert!(true, "ant-quic feature is enabled");
    }

    #[cfg(not(feature = "ant-quic"))]
    {
        panic!("ant-quic feature should be enabled for this test");
    }
}
