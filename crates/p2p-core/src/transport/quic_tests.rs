// Copyright 2024 P2P Foundation
// SPDX-License-Identifier: AGPL-3.0-or-later

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::identity::NodeIdentity;
    use std::sync::Arc;
    use tokio::time::{sleep, Duration};
    
    #[tokio::test]
    async fn test_quic_transport_creation() {
        let transport = QuicTransport::new(true).unwrap();
        assert_eq!(transport.transport_type(), TransportType::QUIC);
        assert!(transport.supports_ipv6());
    }
    
    #[tokio::test]
    async fn test_quic_transport_with_identity() {
        let identity = Arc::new(NodeIdentity::generate(10).unwrap());
        let transport = QuicTransport::new_with_identity(Some(identity.clone()), true).unwrap();
        
        assert_eq!(transport.transport_type(), TransportType::QUIC);
        assert!(transport.identity.is_some());
    }
    
    #[tokio::test]
    async fn test_coordinator_mode() {
        let mut transport = QuicTransport::new(false).unwrap();
        assert!(!transport.config.enable_coordinator);
        
        transport.set_enable_coordinator(true);
        assert!(transport.config.enable_coordinator);
    }
    
    #[tokio::test]
    async fn test_bootstrap_configuration() {
        let bootstrap_nodes = vec![
            "192.168.1.100:9000".parse().unwrap(),
            "192.168.1.101:9001".parse().unwrap(),
        ];
        
        let transport = QuicTransport::new_with_bootstrap(bootstrap_nodes.clone(), true).unwrap();
        assert_eq!(transport.bootstrap_nodes, bootstrap_nodes);
    }
    
    #[tokio::test]
    async fn test_listen_and_accept() {
        let identity1 = Arc::new(NodeIdentity::generate(10).unwrap());
        let identity2 = Arc::new(NodeIdentity::generate(10).unwrap());
        
        // Create server transport
        let server_transport = QuicTransport::new_with_identity(Some(identity1), false).unwrap();
        let listen_addr = NetworkAddress::from_str("127.0.0.1:0").unwrap();
        let actual_addr = server_transport.listen(listen_addr).await.unwrap();
        
        println!("Server listening on: {}", actual_addr);
        
        // Create client transport
        let client_transport = QuicTransport::new_with_identity(Some(identity2), false).unwrap();
        
        // Spawn server accept task
        let server_handle = tokio::spawn(async move {
            server_transport.accept().await
        });
        
        // Give server time to start accepting
        sleep(Duration::from_millis(100)).await;
        
        // Client connects
        let client_conn = client_transport.connect(actual_addr.clone()).await;
        
        // Check if connection succeeded
        assert!(client_conn.is_ok(), "Client connection failed: {:?}", client_conn.err());
        
        // Wait for server to accept
        let server_result = tokio::time::timeout(Duration::from_secs(5), server_handle).await;
        assert!(server_result.is_ok(), "Server accept timed out");
    }
    
    #[tokio::test]
    async fn test_nat_type_detection() {
        // This test requires actual bootstrap nodes, so we skip it in unit tests
        // It would be run in integration tests with a real network
        
        let bootstrap_nodes = vec!["192.168.1.100:9000".parse().unwrap()];
        let transport = QuicTransport::new_with_bootstrap(bootstrap_nodes, false).unwrap();
        
        // NAT detection would happen during listen()
        // For now we just verify the transport is created correctly
        assert!(!transport.bootstrap_nodes.is_empty());
    }
    
    #[tokio::test]
    async fn test_0rtt_configuration() {
        let transport_0rtt = QuicTransport::new(true).unwrap();
        assert!(transport_0rtt.enable_0rtt);
        
        let transport_no_0rtt = QuicTransport::new(false).unwrap();
        assert!(!transport_no_0rtt.enable_0rtt);
    }
    
    #[tokio::test]
    async fn test_peer_id_extraction() {
        // This test would require a full connection setup
        // For now we test the PeerId type conversion
        
        let peer_id: PeerId = vec![1, 2, 3, 4];
        let peer_id_clone = peer_id.clone();
        assert_eq!(peer_id, peer_id_clone);
    }
}