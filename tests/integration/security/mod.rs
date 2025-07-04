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

//! Security module integration tests
//!
//! Comprehensive tests for security functionality including:
//! - Cryptographic key management
//! - Message signing and verification
//! - Peer authentication and authorization
//! - Secure communication channels
//! - Attack resistance and security properties

use anyhow::Result;
use std::time::Duration;
use tokio::time::timeout;

use p2p_foundation::{P2PNode};
use crate::common::{TestNetwork};

// Integration test submodules - TBD
// mod encryption;
// mod authentication;
// mod key_management;
// mod attack_resistance;
// mod certificates;

/// Test basic cryptographic key generation and management
#[tokio::test]
async fn test_key_generation() -> Result<()> {
    let node = P2PNode::new(NodeConfig::default()).await?;
    
    // Test Ed25519 keypair generation
    let ed25519_keypair = node.security().generate_ed25519_keypair().await?;
    assert_eq!(ed25519_keypair.public_key().len(), 32);
    assert_eq!(ed25519_keypair.private_key().len(), 32);
    
    // Test X25519 keypair generation  
    let x25519_keypair = node.security().generate_x25519_keypair().await?;
    assert_eq!(x25519_keypair.public_key().len(), 32);
    assert_eq!(x25519_keypair.private_key().len(), 32);
    
    // Test key derivation
    let derived_key = node.security().derive_key(
        b"test_salt",
        b"test_context",
        32
    ).await?;
    assert_eq!(derived_key.len(), 32);
    
    // Test that derived keys are deterministic
    let derived_key2 = node.security().derive_key(
        b"test_salt",
        b"test_context", 
        32
    ).await?;
    assert_eq!(derived_key, derived_key2);
    
    // Test that different contexts produce different keys
    let different_key = node.security().derive_key(
        b"test_salt",
        b"different_context",
        32
    ).await?;
    assert_ne!(derived_key, different_key);
    
    node.stop().await?;
    Ok(())
}

/// Test message signing and verification
#[tokio::test]
async fn test_message_signing() -> Result<()> {
    let node = P2PNode::new(NodeConfig::default()).await?;
    let security = node.security();
    
    let keypair = security.generate_ed25519_keypair().await?;
    let message = b"test message for signing";
    
    // Sign message
    let signature = security.sign_message(&keypair, message).await?;
    assert_eq!(signature.len(), 64); // Ed25519 signature length
    
    // Verify signature
    let verification = security.verify_signature(
        keypair.public_key(),
        message,
        &signature
    ).await?;
    assert!(verification.is_valid());
    
    // Test invalid signature detection
    let invalid_signature = vec![0u8; 64];
    let invalid_verification = security.verify_signature(
        keypair.public_key(),
        message,
        &invalid_signature
    ).await?;
    assert!(!invalid_verification.is_valid());
    
    // Test signature with wrong message
    let wrong_message = b"different message";
    let wrong_verification = security.verify_signature(
        keypair.public_key(),
        wrong_message,
        &signature
    ).await?;
    assert!(!wrong_verification.is_valid());
    
    node.stop().await?;
    Ok(())
}

/// Test peer authentication
#[tokio::test]
async fn test_peer_authentication() -> Result<()> {
    let network = TestNetwork::simple(2).await?;
    
    let node1_id = network.node(1)?.peer_id();
    let node2_id = network.node(0)?.peer_id();
    
    // Test mutual authentication
    let auth_challenge = network.node(0)?.security().create_auth_challenge(&node1_id).await?;
    let auth_response = network.node(1)?.security().respond_to_auth_challenge(auth_challenge).await?;
    let auth_result = network.node(0)?.security().verify_auth_response(auth_response).await?;
    
    assert!(auth_result.is_authenticated());
    assert_eq!(auth_result.peer_id(), node1_id);
    
    // Test authentication with invalid response
    let mut invalid_response = auth_response.clone();
    invalid_response.signature[0] ^= 0xFF; // Corrupt signature
    
    let invalid_result = network.node(0)?.security().verify_auth_response(invalid_response).await;
    assert!(invalid_result.is_err() || !invalid_result.unwrap().is_authenticated());
    
    // Test bidirectional authentication
    let reverse_challenge = network.node(1)?.security().create_auth_challenge(&node2_id).await?;
    let reverse_response = network.node(0)?.security().respond_to_auth_challenge(reverse_challenge).await?;
    let reverse_result = network.node(1)?.security().verify_auth_response(reverse_response).await?;
    
    assert!(reverse_result.is_authenticated());
    assert_eq!(reverse_result.peer_id(), node2_id);
    
    network.stop().await?;
    Ok(())
}

/// Test secure message encryption and decryption
#[tokio::test]
async fn test_message_encryption() -> Result<()> {
    let network = TestNetwork::simple(2).await?;
    
    let node1_id = network.node(1)?.peer_id();
    let test_message = b"secret message that should be encrypted";
    
    // Get public key for encryption
    let node1_public_key = network.node(1)?.security().get_public_key().await?;
    
    // Encrypt message from node 0 to node 1
    let encrypted_message = network.node(0)?.security().encrypt_message(
        &node1_public_key,
        test_message
    ).await?;
    
    // Verify message is actually encrypted (different from original)
    assert_ne!(encrypted_message.ciphertext, test_message);
    assert!(encrypted_message.nonce.len() > 0);
    assert!(encrypted_message.auth_tag.len() > 0);
    
    // Decrypt message on node 1
    let decrypted_message = network.node(1)?.security().decrypt_message(
        &network.node(0)?.peer_id(),
        &encrypted_message
    ).await?;
    
    assert_eq!(decrypted_message, test_message);
    
    // Test that node 1 cannot decrypt with wrong private key
    let wrong_decryption = network.node(0)?.security().decrypt_message(
        &node1_id,
        &encrypted_message
    ).await;
    assert!(wrong_decryption.is_err());
    
    network.stop().await?;
    Ok(())
}

/// Test secure channel establishment
#[tokio::test]
async fn test_secure_channel() -> Result<()> {
    let network = TestNetwork::simple(2).await?;
    
    let node1_id = network.node(1)?.peer_id();
    
    // Establish secure channel
    let channel_request = network.node(0)?.security().request_secure_channel(&node1_id).await?;
    let channel_response = network.node(1)?.security().accept_secure_channel(channel_request).await?;
    let secure_channel = network.node(0)?.security().complete_secure_channel(channel_response).await?;
    
    // Verify channel properties
    assert!(secure_channel.is_established());
    assert_eq!(secure_channel.peer_id(), node1_id);
    assert!(secure_channel.session_key().len() > 0);
    
    // Test message transmission over secure channel
    let channel_message = b"message over secure channel";
    let transmitted_data = secure_channel.encrypt_data(channel_message).await?;
    
    // Get corresponding channel on node 1
    let node1_channel = network.node(1)?.security().get_secure_channel(&network.node(0)?.peer_id()).await?;
    let received_message = node1_channel.decrypt_data(&transmitted_data).await?;
    
    assert_eq!(received_message, channel_message);
    
    // Test key rotation
    let old_session_key = secure_channel.session_key().clone();
    secure_channel.rotate_keys().await?;
    let new_session_key = secure_channel.session_key();
    
    assert_ne!(old_session_key, *new_session_key);
    
    // Verify communication still works after key rotation
    let post_rotation_message = b"message after key rotation";
    let post_rotation_data = secure_channel.encrypt_data(post_rotation_message).await?;
    let post_rotation_received = node1_channel.decrypt_data(&post_rotation_data).await?;
    
    assert_eq!(post_rotation_received, post_rotation_message);
    
    network.stop().await?;
    Ok(())
}

/// Test certificate-based authentication
#[tokio::test]
async fn test_certificate_authentication() -> Result<()> {
    // Create certificate authority
    let ca_config = TestNodeConfig::builder()
        .port(9400)
        .build();
    let mut ca_node_config = ca_config;
    ca_node_config.enable_certificate_authority = true;
    
    let ca_node = P2PNode::new(ca_node_config).await?;
    let ca_cert = ca_node.security().get_ca_certificate().await?;
    
    // Create nodes with certificates issued by CA
    let config1 = TestNodeConfig::builder()
        .port(9401)
        .build();
    let mut cert_config1 = config1;
    cert_config1.ca_certificate = Some(ca_cert.clone());
    cert_config1.require_certificates = true;
    
    let config2 = TestNodeConfig::builder()
        .port(9402)
        .build();
    let mut cert_config2 = config2;
    cert_config2.ca_certificate = Some(ca_cert.clone());
    cert_config2.require_certificates = true;
    
    let node1 = P2PNode::new(cert_config1).await?;
    let node2 = P2PNode::new(cert_config2).await?;
    
    // Request certificates from CA
    let cert_request1 = node1.security().create_certificate_request("node1").await?;
    let cert1 = ca_node.security().issue_certificate(cert_request1).await?;
    node1.security().install_certificate(cert1).await?;
    
    let cert_request2 = node2.security().create_certificate_request("node2").await?;
    let cert2 = ca_node.security().issue_certificate(cert_request2).await?;
    node2.security().install_certificate(cert2).await?;
    
    // Connect nodes - should use certificate authentication
    let node1_addrs = node1.listen_addrs().await?;
    node2.connect(node1_addrs[0].clone()).await?;
    
    // Verify certificate-based authentication was used
    let node1_id = node1.peer_id();
    let connection_info = node2.get_connection_info(&node1_id).await?;
    assert!(connection_info.uses_certificates);
    assert!(connection_info.certificate_verified);
    
    // Test that nodes without valid certificates are rejected
    let unauth_config = TestNodeConfig::builder()
        .port(9403)
        .build();
    let unauth_node = P2PNode::new(unauth_config).await?;
    
    let unauth_addrs = unauth_node.listen_addrs().await?;
    let connection_result = timeout(
        Duration::from_secs(5),
        node1.connect(unauth_addrs[0].clone())
    ).await;
    
    // Should fail due to missing certificate
    assert!(connection_result.is_err() || connection_result.unwrap().is_err());
    
    // Cleanup
    ca_node.stop().await?;
    node1.stop().await?;
    node2.stop().await?;
    unauth_node.stop().await?;
    
    Ok(())
}

/// Test protection against replay attacks
#[tokio::test]
async fn test_replay_attack_protection() -> Result<()> {
    let network = TestNetwork::simple(2).await?;
    
    let node1_id = network.node(1)?.peer_id();
    let test_message = b"message that could be replayed";
    
    // Send authentic message
    let signed_message = network.node(0)?.security().create_signed_message(
        &node1_id,
        test_message
    ).await?;
    
    // First delivery should succeed
    let first_result = network.node(1)?.security().verify_signed_message(&signed_message).await?;
    assert!(first_result.is_valid());
    assert!(!first_result.is_replay());
    
    // Attempt to replay the same message - should be detected
    let replay_result = network.node(1)?.security().verify_signed_message(&signed_message).await?;
    assert!(!replay_result.is_valid() || replay_result.is_replay());
    
    // Test with modified timestamp (should also fail)
    let mut modified_message = signed_message.clone();
    modified_message.timestamp += Duration::from_secs(3600); // 1 hour later
    
    let modified_result = network.node(1)?.security().verify_signed_message(&modified_message).await?;
    assert!(!modified_result.is_valid());
    
    network.stop().await?;
    Ok(())
}

/// Test protection against man-in-the-middle attacks
#[tokio::test]
async fn test_mitm_protection() -> Result<()> {
    let network = TestNetwork::simple(3).await?;
    
    // Nodes 0 and 1 are legitimate, node 2 is attempting MITM
    let node1_id = network.node(1)?.peer_id();
    let attacker_id = network.node(2)?.peer_id();
    
    // Establish secure channel between nodes 0 and 1
    let channel_request = network.node(0)?.security().request_secure_channel(&node1_id).await?;
    
    // Attacker tries to intercept and modify the channel establishment
    let mut intercepted_request = channel_request.clone();
    intercepted_request.ephemeral_public_key = network.node(2)?.security().get_public_key().await?.clone();
    
    // Node 1 should detect the tampering
    let tampered_response = network.node(1)?.security().accept_secure_channel(intercepted_request).await;
    assert!(tampered_response.is_err(), "Should detect tampered channel request");
    
    // Legitimate channel establishment should still work
    let legitimate_response = network.node(1)?.security().accept_secure_channel(channel_request).await?;
    let secure_channel = network.node(0)?.security().complete_secure_channel(legitimate_response).await?;
    
    // Verify the channel is established with the correct peer
    assert_eq!(secure_channel.peer_id(), node1_id);
    assert_ne!(secure_channel.peer_id(), attacker_id);
    
    // Test that attacker cannot decrypt channel traffic
    let secret_message = b"secret communication";
    let encrypted_data = secure_channel.encrypt_data(secret_message).await?;
    
    let attacker_channel_result = network.node(2)?.security().get_secure_channel(&network.node(0)?.peer_id()).await;
    assert!(attacker_channel_result.is_err(), "Attacker should not have access to secure channel");
    
    network.stop().await?;
    Ok(())
}

/// Test rate limiting and DoS protection
#[tokio::test]
async fn test_dos_protection() -> Result<()> {
    let config1 = TestNodeConfig::builder()
        .port(9410)
        .build();
    let mut protected_config = config1;
    protected_config.enable_rate_limiting = true;
    protected_config.max_requests_per_second = 10;
    protected_config.max_connections_per_peer = 2;
    
    let config2 = TestNodeConfig::builder()
        .port(9411)
        .build();
    
    let protected_node = P2PNode::new(protected_config).await?;
    let client_node = P2PNode::new(config2).await?;
    
    let protected_addrs = protected_node.listen_addrs().await?;
    client_node.connect(protected_addrs[0].clone()).await?;
    
    let protected_id = protected_node.peer_id();
    
    // Test rate limiting
    let mut successful_requests = 0;
    let mut rate_limited_requests = 0;
    
    // Send requests rapidly to trigger rate limiting
    for i in 0..20 {
        let request_data = format!("request_{}", i).into_bytes();
        let start = std::time::Instant::now();
        
        let result = client_node.send_message(&protected_id, request_data).await;
        
        match result {
            Ok(_) => {
                successful_requests += 1;
                // Small delay to avoid overwhelming
                tokio::time::sleep(Duration::from_millis(50)).await;
            },
            Err(e) if e.to_string().contains("rate limit") => {
                rate_limited_requests += 1;
            },
            Err(e) => {
                println!("Unexpected error: {}", e);
            }
        }
    }
    
    println!("Successful requests: {}, Rate limited: {}", successful_requests, rate_limited_requests);
    
    // Should have some rate limiting
    assert!(rate_limited_requests > 0, "Rate limiting should have triggered");
    assert!(successful_requests > 0, "Some requests should have succeeded");
    
    // Test connection limiting
    let mut additional_connections = Vec::new();
    
    for i in 0..5 {
        let extra_config = TestNodeConfig::builder()
            .port(9412 + i as u16)
            .build();
        let extra_node = P2PNode::new(extra_config).await?;
        
        let connection_result = timeout(
            Duration::from_secs(2),
            extra_node.connect(protected_addrs[0].clone())
        ).await;
        
        match connection_result {
            Ok(Ok(_)) => {
                additional_connections.push(extra_node);
            },
            Ok(Err(_)) | Err(_) => {
                // Connection rejected due to limits
                extra_node.stop().await?;
                break;
            }
        }
    }
    
    // Should reject connections beyond the limit
    assert!(
        additional_connections.len() <= 2,
        "Should enforce connection limits, got {} additional connections",
        additional_connections.len()
    );
    
    // Cleanup
    protected_node.stop().await?;
    client_node.stop().await?;
    for node in additional_connections {
        node.stop().await?;
    }
    
    Ok(())
}

/// Test secure key exchange under adversarial conditions
#[tokio::test]
async fn test_adversarial_key_exchange() -> Result<()> {
    let network = TestNetwork::simple(3).await?;
    
    // Nodes 0 and 1 perform key exchange while node 2 observes
    let node1_id = network.node(1)?.peer_id();
    let observer_id = network.node(2)?.peer_id();
    
    // Perform Diffie-Hellman key exchange
    let dh_private_0 = network.node(0)?.security().generate_dh_private_key().await?;
    let dh_public_0 = network.node(0)?.security().compute_dh_public_key(&dh_private_0).await?;
    
    let dh_private_1 = network.node(1)?.security().generate_dh_private_key().await?;
    let dh_public_1 = network.node(1)?.security().compute_dh_public_key(&dh_private_1).await?;
    
    // Exchange public keys (observer can see these)
    let shared_secret_0 = network.node(0)?.security().compute_dh_shared_secret(&dh_private_0, &dh_public_1).await?;
    let shared_secret_1 = network.node(1)?.security().compute_dh_shared_secret(&dh_private_1, &dh_public_0).await?;
    
    // Verify both nodes computed the same shared secret
    assert_eq!(shared_secret_0, shared_secret_1);
    
    // Observer should not be able to compute the shared secret from public keys alone
    let observer_attempt = network.node(2)?.security().attempt_dh_attack(&dh_public_0, &dh_public_1).await;
    assert!(observer_attempt.is_err(), "Observer should not be able to break DH key exchange");
    
    // Test forward secrecy - compromise of long-term keys shouldn't affect session
    let compromised_private = network.node(0)?.security().get_private_key().await?;
    
    // Even with compromised private key, attacker shouldn't get session key
    let forward_secrecy_attack = network.node(2)?.security().break_forward_secrecy(
        &compromised_private,
        &dh_public_0,
        &dh_public_1
    ).await;
    
    assert!(forward_secrecy_attack.is_err(), "Should maintain forward secrecy");
    
    network.stop().await?;
    Ok(())
}

/// Test cryptographic strength and randomness
#[tokio::test]
async fn test_cryptographic_strength() -> Result<()> {
    let node = P2PNode::new(NodeConfig::default()).await?;
    let security = node.security();
    
    // Test randomness quality
    let mut random_samples = Vec::new();
    for _ in 0..100 {
        let random_bytes = security.generate_random_bytes(32).await?;
        random_samples.push(random_bytes);
    }
    
    // Check that all samples are different (extremely high probability with good randomness)
    for i in 0..random_samples.len() {
        for j in (i + 1)..random_samples.len() {
            assert_ne!(
                random_samples[i], random_samples[j],
                "Random samples should be unique"
            );
        }
    }
    
    // Test entropy estimation
    let entropy_estimate = security.estimate_entropy(&random_samples[0]).await?;
    assert!(
        entropy_estimate > 7.5, // Should be close to 8.0 for good randomness
        "Entropy too low: {}",
        entropy_estimate
    );
    
    // Test key strength
    let weak_key_test = security.test_key_strength(&random_samples[0]).await?;
    assert!(weak_key_test.is_strong());
    assert!(!weak_key_test.has_patterns());
    assert!(!weak_key_test.is_weak());
    
    // Test that known weak keys are detected
    let weak_key = vec![0u8; 32]; // All zeros
    let weak_test = security.test_key_strength(&weak_key).await?;
    assert!(!weak_test.is_strong());
    assert!(weak_test.is_weak());
    
    node.stop().await?;
    Ok(())
}