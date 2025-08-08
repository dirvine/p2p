//! Comprehensive Security Integration Tests
//!
//! Tests authentication flows, encryption verification, attack simulations,
//! and access control mechanisms.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{Duration, sleep};

use saorsa_core::{
    Config, Identity, Node, SecureMessage,
    crypto_verify::CryptoVerifier,
    identity::IdentityManager,
    security::{EncryptionKey, SecurityManager, SigningKey},
};

/// Test framework for security scenarios
struct SecurityTestFramework {
    nodes: Vec<Arc<Node>>,
    identities: Vec<Identity>,
    malicious_nodes: Vec<Arc<Node>>,
}

impl SecurityTestFramework {
    async fn new(node_count: usize, malicious_count: usize) -> Result<Self> {
        let mut nodes = Vec::new();
        let mut identities = Vec::new();
        let mut malicious_nodes = Vec::new();

        // Create legitimate nodes
        for i in 0..node_count {
            let mut config = Config::default();
            config.network.listen_port = 7000 + i as u16;
            config.security.enable_encryption = true;
            config.security.require_authentication = true;
            config.security.max_failed_auth_attempts = 3;

            let identity = Identity::generate()?;
            let node = Node::new_with_identity(config, identity.clone())
                .await
                .context(format!("Failed to create secure node {}", i))?;

            nodes.push(Arc::new(node));
            identities.push(identity);
        }

        // Create malicious nodes (for attack simulation)
        for i in 0..malicious_count {
            let mut config = Config::default();
            config.network.listen_port = 6000 + i as u16;
            config.security.enable_encryption = false; // Malicious: try to bypass security

            let identity = Identity::generate()?;
            let node = Node::new_with_identity(config, identity)
                .await
                .context(format!("Failed to create malicious node {}", i))?;

            malicious_nodes.push(Arc::new(node));
        }

        Ok(Self {
            nodes,
            identities,
            malicious_nodes,
        })
    }

    async fn start_legitimate_nodes(&self) -> Result<()> {
        for (i, node) in self.nodes.iter().enumerate() {
            node.start()
                .await
                .context(format!("Failed to start secure node {}", i))?;
            sleep(Duration::from_millis(100)).await;
        }

        // Connect legitimate nodes securely
        for i in 0..self.nodes.len() {
            for j in (i + 1)..self.nodes.len() {
                let peer_addr = format!("/ip4/127.0.0.1/tcp/{}", 7000 + j);
                self.nodes[i]
                    .connect_to_peer_secure(&peer_addr, &self.identities[j])
                    .await?;
                sleep(Duration::from_millis(50)).await;
            }
        }

        sleep(Duration::from_secs(3)).await;
        Ok(())
    }

    async fn start_malicious_nodes(&self) -> Result<()> {
        for (i, node) in self.malicious_nodes.iter().enumerate() {
            node.start()
                .await
                .context(format!("Failed to start malicious node {}", i))?;
            sleep(Duration::from_millis(100)).await;
        }
        Ok(())
    }

    async fn simulate_authentication_attack(&self) -> Result<Vec<bool>> {
        let mut attack_results = Vec::new();

        for (i, malicious_node) in self.malicious_nodes.iter().enumerate() {
            // Try to connect to legitimate nodes without proper authentication
            for j in 0..self.nodes.len() {
                let peer_addr = format!("/ip4/127.0.0.1/tcp/{}", 7000 + j);

                // This should fail due to authentication requirements
                let connection_result = malicious_node.connect_to_peer(&peer_addr).await;
                let attack_succeeded = connection_result.is_ok();

                attack_results.push(attack_succeeded);

                if attack_succeeded {
                    println!(
                        "⚠️ Authentication bypass detected from malicious node {}",
                        i
                    );
                }
            }
        }

        Ok(attack_results)
    }

    async fn test_message_encryption(&self) -> Result<bool> {
        if self.nodes.len() < 2 {
            return Ok(true);
        }

        let sender = &self.nodes[0];
        let receiver = &self.nodes[1];

        // Send encrypted message
        let message = b"sensitive_test_data".to_vec();
        let encrypted_msg = sender
            .encrypt_message(&message, &self.identities[1])
            .await?;

        // Verify message is actually encrypted (not plaintext)
        let is_encrypted = encrypted_msg.ciphertext != message;

        // Send the encrypted message
        sender
            .send_encrypted_message(&self.identities[1].peer_id(), encrypted_msg.clone())
            .await?;

        // Receiver should be able to decrypt
        let decrypted = receiver
            .decrypt_message(encrypted_msg, &self.identities[0])
            .await?;
        let decryption_successful = decrypted == message;

        Ok(is_encrypted && decryption_successful)
    }

    async fn simulate_man_in_the_middle_attack(&self) -> Result<bool> {
        if self.nodes.len() < 2 || self.malicious_nodes.is_empty() {
            return Ok(true);
        }

        let sender = &self.nodes[0];
        let receiver = &self.nodes[1];
        let attacker = &self.malicious_nodes[0];

        // Normal encrypted communication
        let message = b"confidential_data".to_vec();
        let encrypted_msg = sender
            .encrypt_message(&message, &self.identities[1])
            .await?;

        // Attacker tries to intercept and modify message
        let modified_msg = attacker.attempt_message_modification(encrypted_msg).await;

        // Receiver attempts to decrypt potentially modified message
        match receiver
            .decrypt_message(modified_msg, &self.identities[0])
            .await
        {
            Ok(decrypted) => {
                // If decryption succeeds, check if message was tampered
                let tamper_detected = decrypted != message;
                Ok(tamper_detected) // Return true if tampering was detected
            }
            Err(_) => {
                // Decryption failed - good, tampering detected
                Ok(true)
            }
        }
    }

    async fn test_replay_attack_protection(&self) -> Result<bool> {
        if self.nodes.len() < 2 {
            return Ok(true);
        }

        let sender = &self.nodes[0];
        let receiver = &self.nodes[1];

        // Send a legitimate message
        let message = b"legitimate_request".to_vec();
        let signed_msg = sender.sign_message(&message).await?;

        // First send should succeed
        let first_result = receiver
            .verify_and_process_message(signed_msg.clone())
            .await;
        let first_success = first_result.is_ok();

        // Replay the same message (should be rejected)
        sleep(Duration::from_millis(100)).await;
        let replay_result = receiver.verify_and_process_message(signed_msg).await;
        let replay_rejected = replay_result.is_err();

        Ok(first_success && replay_rejected)
    }

    async fn test_access_control(&self) -> Result<Vec<bool>> {
        let mut access_results = Vec::new();

        if self.nodes.is_empty() {
            return Ok(access_results);
        }

        let admin_node = &self.nodes[0];

        // Set up access control rules
        admin_node
            .set_access_policy("admin_resource", vec![self.identities[0].peer_id()])
            .await?;
        admin_node
            .set_access_policy(
                "public_resource",
                self.identities.iter().map(|id| id.peer_id()).collect(),
            )
            .await?;

        // Test legitimate access
        for (i, node) in self.nodes.iter().enumerate() {
            // Admin resource - only first node should have access
            let admin_access = node.access_resource("admin_resource").await.is_ok();
            let expected_admin_access = i == 0;
            access_results.push(admin_access == expected_admin_access);

            // Public resource - all nodes should have access
            let public_access = node.access_resource("public_resource").await.is_ok();
            access_results.push(public_access);
        }

        // Test malicious access attempts
        for malicious_node in &self.malicious_nodes {
            let admin_access = malicious_node
                .access_resource("admin_resource")
                .await
                .is_ok();
            let public_access = malicious_node
                .access_resource("public_resource")
                .await
                .is_ok();

            // Malicious nodes should be denied access
            access_results.push(!admin_access);
            access_results.push(!public_access);
        }

        Ok(access_results)
    }

    async fn performance_benchmark_encryption(&self) -> Result<(f64, f64)> {
        if self.nodes.len() < 2 {
            return Ok((0.0, 0.0));
        }

        let sender = &self.nodes[0];
        let receiver = &self.nodes[1];

        let message_sizes = vec![1024, 10240, 102400]; // 1KB, 10KB, 100KB
        let iterations = 10;

        let mut total_encrypt_time = 0.0;
        let mut total_decrypt_time = 0.0;
        let mut total_operations = 0;

        for size in message_sizes {
            let message = vec![0xCD; size];

            for _ in 0..iterations {
                // Measure encryption time
                let encrypt_start = std::time::Instant::now();
                let encrypted = sender
                    .encrypt_message(&message, &self.identities[1])
                    .await?;
                let encrypt_duration = encrypt_start.elapsed();

                // Measure decryption time
                let decrypt_start = std::time::Instant::now();
                let _ = receiver
                    .decrypt_message(encrypted, &self.identities[0])
                    .await?;
                let decrypt_duration = decrypt_start.elapsed();

                total_encrypt_time += encrypt_duration.as_secs_f64();
                total_decrypt_time += decrypt_duration.as_secs_f64();
                total_operations += 1;
            }
        }

        let avg_encrypt_time = total_encrypt_time / total_operations as f64;
        let avg_decrypt_time = total_decrypt_time / total_operations as f64;

        Ok((avg_encrypt_time * 1000.0, avg_decrypt_time * 1000.0)) // Return in milliseconds
    }

    async fn shutdown_all(&self) -> Result<()> {
        for node in &self.nodes {
            let _ = node.shutdown().await;
        }
        for node in &self.malicious_nodes {
            let _ = node.shutdown().await;
        }
        Ok(())
    }
}

#[tokio::test]
async fn test_secure_node_authentication() -> Result<()> {
    let framework = SecurityTestFramework::new(3, 1).await?;

    framework.start_legitimate_nodes().await?;
    framework.start_malicious_nodes().await?;

    // Test that malicious nodes cannot authenticate
    let attack_results = framework.simulate_authentication_attack().await?;

    // All authentication attacks should fail
    for (i, succeeded) in attack_results.iter().enumerate() {
        assert!(!succeeded, "Authentication attack {} should have failed", i);
    }

    // Verify legitimate nodes are connected
    for node in &framework.nodes {
        let peer_count = node.get_connected_peers().await?.len();
        assert!(peer_count >= 1, "Legitimate nodes should be connected");
    }

    framework.shutdown_all().await?;
    Ok(())
}

#[tokio::test]
async fn test_message_encryption_integrity() -> Result<()> {
    let framework = SecurityTestFramework::new(3, 0).await?;
    framework.start_legitimate_nodes().await?;

    // Test message encryption
    let encryption_works = framework.test_message_encryption().await?;
    assert!(encryption_works, "Message encryption should work correctly");

    // Test multiple message exchanges
    for i in 0..5 {
        let sender_idx = i % framework.nodes.len();
        let receiver_idx = (i + 1) % framework.nodes.len();

        let message = format!("test_message_{}", i).into_bytes();
        let encrypted = framework.nodes[sender_idx]
            .encrypt_message(&message, &framework.identities[receiver_idx])
            .await?;

        let decrypted = framework.nodes[receiver_idx]
            .decrypt_message(encrypted, &framework.identities[sender_idx])
            .await?;

        assert_eq!(
            decrypted, message,
            "Message {} encryption/decryption failed",
            i
        );
    }

    framework.shutdown_all().await?;
    Ok(())
}

#[tokio::test]
async fn test_attack_resistance() -> Result<()> {
    let framework = SecurityTestFramework::new(3, 2).await?;

    framework.start_legitimate_nodes().await?;
    framework.start_malicious_nodes().await?;

    // Test man-in-the-middle attack detection
    let mitm_detected = framework.simulate_man_in_the_middle_attack().await?;
    assert!(mitm_detected, "Man-in-the-middle attack should be detected");

    // Test replay attack protection
    let replay_protected = framework.test_replay_attack_protection().await?;
    assert!(replay_protected, "Replay attacks should be prevented");

    framework.shutdown_all().await?;
    Ok(())
}

#[tokio::test]
async fn test_access_control_enforcement() -> Result<()> {
    let framework = SecurityTestFramework::new(4, 2).await?;

    framework.start_legitimate_nodes().await?;
    framework.start_malicious_nodes().await?;

    let access_results = framework.test_access_control().await?;

    // All access control tests should pass
    for (i, result) in access_results.iter().enumerate() {
        assert!(result, "Access control test {} failed", i);
    }

    framework.shutdown_all().await?;
    Ok(())
}

#[tokio::test]
async fn test_crypto_performance() -> Result<()> {
    let framework = SecurityTestFramework::new(2, 0).await?;
    framework.start_legitimate_nodes().await?;

    let (avg_encrypt_ms, avg_decrypt_ms) = framework.performance_benchmark_encryption().await?;

    println!("Encryption performance: {:.2}ms avg", avg_encrypt_ms);
    println!("Decryption performance: {:.2}ms avg", avg_decrypt_ms);

    // Performance assertions (adjust based on requirements)
    assert!(
        avg_encrypt_ms < 50.0,
        "Encryption should be < 50ms on average"
    );
    assert!(
        avg_decrypt_ms < 50.0,
        "Decryption should be < 50ms on average"
    );

    framework.shutdown_all().await?;
    Ok(())
}

#[tokio::test]
async fn test_identity_verification() -> Result<()> {
    let framework = SecurityTestFramework::new(3, 0).await?;
    framework.start_legitimate_nodes().await?;

    // Test identity verification between all node pairs
    for i in 0..framework.nodes.len() {
        for j in 0..framework.nodes.len() {
            if i != j {
                let verifier = &framework.nodes[i];
                let identity_to_verify = &framework.identities[j];

                // Create a signed message from node j
                let message = format!("identity_test_{}_{}", i, j).into_bytes();
                let signature = framework.nodes[j].sign_message(&message).await?;

                // Node i should be able to verify the signature
                let verification_result = verifier
                    .verify_signature(&message, &signature, &identity_to_verify.public_key())
                    .await?;

                assert!(
                    verification_result,
                    "Node {} should verify identity of node {}",
                    i, j
                );
            }
        }
    }

    framework.shutdown_all().await?;
    Ok(())
}

#[tokio::test]
async fn test_secure_group_communication() -> Result<()> {
    let framework = SecurityTestFramework::new(5, 0).await?;
    framework.start_legitimate_nodes().await?;

    // Create a secure group
    let group_members: Vec<_> = framework.identities.iter().take(3).collect();
    let group_key = framework.nodes[0].create_group_key(&group_members).await?;

    // Distribute group key to members
    for i in 1..3 {
        framework.nodes[0]
            .share_group_key(&framework.identities[i], &group_key)
            .await?;
    }

    // Test group message encryption/decryption
    let group_message = b"secret_group_message".to_vec();
    let encrypted_group_msg = framework.nodes[0]
        .encrypt_group_message(&group_message, &group_key)
        .await?;

    // All group members should be able to decrypt
    for i in 0..3 {
        let decrypted = framework.nodes[i]
            .decrypt_group_message(&encrypted_group_msg, &group_key)
            .await?;
        assert_eq!(
            decrypted, group_message,
            "Group member {} should decrypt message",
            i
        );
    }

    // Non-group members should NOT be able to decrypt
    for i in 3..framework.nodes.len() {
        let decrypt_result = framework.nodes[i]
            .decrypt_group_message(&encrypted_group_msg, &group_key)
            .await;
        assert!(
            decrypt_result.is_err(),
            "Non-group member {} should not decrypt message",
            i
        );
    }

    framework.shutdown_all().await?;
    Ok(())
}
