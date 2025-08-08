// Copyright 2024 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::identity::NodeIdentity;
    use crate::NetworkAddress;
    use std::sync::Arc;
    use std::str::FromStr;
    
    #[tokio::test]
    async fn test_quic_transport_creation() {
        let options = TransportOptions {
            enable_server: true,
            enable_0rtt: false,
            bootstrap_nodes: vec![],
        };
        let transport = QuicTransport::new(options).unwrap();
        // Basic creation test passes if no panic
        assert!(true);
    }
    
    #[tokio::test] 
    async fn test_quic_transport_bind() {
        let options = TransportOptions {
            enable_server: true,
            enable_0rtt: false,
            bootstrap_nodes: vec![],
        };
        let mut transport = QuicTransport::new(options).unwrap();
        
        let addr = "127.0.0.1:0".parse::<NetworkAddress>().unwrap();
        let result = transport.bind(addr).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_quic_connection_info() {
        use std::net::{IpAddr, Ipv4Addr};
        
        let local_addr = "127.0.0.1:9000".parse::<NetworkAddress>().unwrap();
        let remote_addr = "127.0.0.1:9001".parse::<NetworkAddress>().unwrap();
        
        // Mock connection for testing info method
        let conn = QuicConnection {
            peer_id: "test-peer".to_string(),
            connection: unsafe { std::mem::zeroed() }, // This is just for testing ConnectionInfo
            local_addr: local_addr.clone(),
            remote_addr: remote_addr.clone(),
            established_at: std::time::Instant::now(),
        };
        
        let info = conn.info();
        assert_eq!(info.transport_type, TransportType::QUIC);
        assert_eq!(info.local_addr, local_addr);
        assert_eq!(info.remote_addr, remote_addr);
        assert!(info.is_encrypted);
        assert!(!info.used_0rtt);
    }
}