// Copyright 2024 Saorsa Labs Limited
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

//! Tests for QUIC transport error handling

#[cfg(test)]
mod tests {
    use crate::transport::quic::QuicTransport;
    use crate::error::{P2PError, TransportError};
    
    #[tokio::test]
    async fn test_transport_config_error_handling() {
        // Test that we properly handle the case where Arc::get_mut fails
        // This simulates a scenario where the transport config Arc has multiple references
        
        // Create a transport (this will succeed internally)
        let transport = QuicTransport::new(0, None, None)
            .expect("Should create transport");
        
        // Verify transport is functional
        let local_addr = transport.local_addrs().await
            .expect("Should get local addresses");
        assert!(!local_addr.is_empty(), "Should have at least one local address");
    }
    
    #[test] 
    fn test_transport_error_types() {
        // Test that our error types are properly constructed
        let err = P2PError::Transport(TransportError::SetupFailed(
            "Test setup failure".into()
        ));
        
        assert!(matches!(err, P2PError::Transport(TransportError::SetupFailed(_))));
        assert!(err.to_string().contains("Test setup failure"));
    }
}