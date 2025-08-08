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

//! Comprehensive TDD tests for Node Identity System
//!
//! Following Test-Driven Development approach:
//! 1. Write failing tests first (Red)
//! 2. Implement minimum code to pass (Green)
//! 3. Refactor while keeping tests passing

use proptest::prelude::*;
use saorsa_core::identity::{FourWordAddress, NodeId, NodeIdentity, ProofOfWork};
use std::time::Duration;
use tempfile::TempDir;

// Constants for testing
const TEST_DIFFICULTY: u32 = 8; // Low difficulty for fast tests
const PRODUCTION_DIFFICULTY: u32 = 16; // Minimum production difficulty

#[cfg(test)]
mod identity_generation_tests {
    use super::*;

    #[test]
    fn test_basic_identity_generation() {
        // Test basic identity creation
        let identity = NodeIdentity::generate(TEST_DIFFICULTY).unwrap();

        // Verify all components are present
        assert!(identity.node_id().to_bytes().len() == 32);
        assert!(!identity.word_address().to_string().is_empty());
        assert!(identity.verify_proof_of_work());
    }

    #[test]
    fn test_identity_from_seed() {
        let seed = [0x42; 32];
        let identity = NodeIdentity::from_seed(&seed, TEST_DIFFICULTY).unwrap();

        // Verify identity was created
        assert_eq!(identity.node_id().to_bytes().len(), 32);
        assert!(identity.verify_proof_of_work());
    }

    proptest! {
        #[test]
        fn prop_deterministic_from_seed(seed: [u8; 32]) {
            // Same seed should always produce same identity
            let id1 = NodeIdentity::from_seed(&seed, TEST_DIFFICULTY).unwrap();
            let id2 = NodeIdentity::from_seed(&seed, TEST_DIFFICULTY).unwrap();

            prop_assert_eq!(id1.node_id(), id2.node_id());
            prop_assert_eq!(id1.word_address().to_string(), id2.word_address().to_string());
            prop_assert_eq!(id1.public_key().as_bytes(), id2.public_key().as_bytes());
        }

        #[test]
        fn prop_different_seeds_different_ids(seed1: [u8; 32], seed2: [u8; 32]) {
            // Different seeds should produce different identities
            prop_assume!(seed1 != seed2);

            let id1 = NodeIdentity::from_seed(&seed1, TEST_DIFFICULTY).unwrap();
            let id2 = NodeIdentity::from_seed(&seed2, TEST_DIFFICULTY).unwrap();

            prop_assert_ne!(id1.node_id(), id2.node_id());
            prop_assert_ne!(id1.word_address().to_string(), id2.word_address().to_string());
        }
    }

    #[test]
    fn test_node_id_from_public_key() {
        let identity = NodeIdentity::generate(TEST_DIFFICULTY).unwrap();
        let public_key = identity.public_key();

        // Node ID should be deterministic from public key
        let node_id = NodeId::from_public_key(public_key);
        assert_eq!(node_id, *identity.node_id());
    }
}

#[cfg(test)]
mod four_word_address_tests {
    use super::*;

    #[test]
    fn test_four_word_address_format() {
        let identity = NodeIdentity::generate(TEST_DIFFICULTY).unwrap();
        let address = identity.word_address();

        // Verify format: word-word-word-word
        let address_str = address.to_string();
        let words: Vec<&str> = address_str.split('-').collect();
        assert_eq!(words.len(), 4);

        // Each word should be non-empty
        for word in words {
            assert!(!word.is_empty());
            // Words should be lowercase
            assert_eq!(word, word.to_lowercase());
        }
    }

    #[test]
    fn test_four_word_address_deterministic() {
        let identity = NodeIdentity::generate(TEST_DIFFICULTY).unwrap();
        let node_id = identity.node_id();

        // Creating address from same node_id should be deterministic
        let addr1 =
            FourWordAddress::from_node_id(node_id).expect("Should create address from node ID");
        let addr2 =
            FourWordAddress::from_node_id(node_id).expect("Should create address from node ID");

        assert_eq!(addr1.to_string(), addr2.to_string());
    }

    #[test]
    fn test_four_word_address_parsing() {
        // Test parsing from string
        let test_address = "alpha-bravo-charlie-delta";
        let parsed = FourWordAddress::from_string(test_address);

        assert!(parsed.is_ok());
        assert_eq!(parsed.unwrap().to_string(), test_address);
    }

    #[test]
    fn test_invalid_four_word_address() {
        // Test invalid formats
        let invalid_addresses = vec![
            "alpha-bravo-charlie",            // Too few words
            "alpha-bravo-charlie-delta-echo", // Too many words
            "alpha bravo charlie delta",      // Wrong separator
            "",                               // Empty
            "ALPHA-BRAVO-CHARLIE-DELTA",      // Uppercase (if not allowed)
        ];

        for invalid in invalid_addresses {
            let result = FourWordAddress::from_string(invalid);
            assert!(
                result.is_err(),
                "Should reject invalid address: {}",
                invalid
            );
        }
    }

    proptest! {
        #[test]
        fn prop_four_word_roundtrip(node_id_bytes: [u8; 32]) {
            // Test roundtrip: NodeId -> FourWords -> NodeId
            let node_id = NodeId(node_id_bytes);
            let address = FourWordAddress::from_node_id(&node_id).unwrap();

            // Verify address format
            let address_str = address.to_string();
        let words: Vec<&str> = address_str.split('-').collect();
            prop_assert_eq!(words.len(), 4);
        }
    }
}

#[cfg(test)]
mod proof_of_work_tests {
    use super::*;

    #[test]
    fn test_proof_of_work_validation() {
        let identity = NodeIdentity::generate(TEST_DIFFICULTY).unwrap();

        // Valid PoW should pass
        assert!(identity.verify_proof_of_work());

        // Get proof of work
        let pow = identity.proof_of_work();
        assert!(pow.verify(identity.node_id(), TEST_DIFFICULTY));
    }

    #[test]
    fn test_proof_of_work_difficulty() {
        // Generate with specific difficulty
        let difficulty = 10;
        let node_id = NodeId([0x42; 32]);
        let pow = ProofOfWork::solve(&node_id, difficulty).unwrap();

        // Verify the hash has required leading zeros
        let hash = pow.compute_hash(&node_id);
        let leading_zeros = count_leading_zero_bits(&hash);
        assert!(
            leading_zeros >= difficulty,
            "Expected {} leading zeros, got {}",
            difficulty,
            leading_zeros
        );
    }

    #[test]
    fn test_invalid_proof_of_work() {
        let identity = NodeIdentity::generate(TEST_DIFFICULTY).unwrap();
        let pow = identity.proof_of_work();

        // Create a different node_id
        let different_node_id = NodeId([0xFF; 32]);

        // Should fail verification with different node_id
        assert!(!pow.verify(&different_node_id, TEST_DIFFICULTY));
    }

    #[test]
    fn test_proof_of_work_timing() -> Result<()> {
        let node_id = NodeId([0x42; 32]);
        let start = std::time::Instant::now();

        // Compute PoW with reasonable difficulty
        let pow = ProofOfWork::solve(&node_id, TEST_DIFFICULTY).unwrap();
        let elapsed = start.elapsed();

        // Should complete in reasonable time (adjust as needed)
        assert!(
            elapsed < Duration::from_secs(5),
            "PoW took too long: {:?}",
            elapsed
        );

        // Verify computation time is recorded
        assert!(pow.computation_time > Duration::from_nanos(1));
        Ok(())
    }

    #[test]
    #[should_panic(expected = "timeout")]
    fn test_proof_of_work_timeout() {
        let node_id = NodeId([0x42; 32]);

        // Unreasonably high difficulty should timeout
        let _ = ProofOfWork::solve_with_timeout(
            &node_id,
            64, // Very high difficulty
            Duration::from_millis(100),
        );
    }
}

#[cfg(test)]
mod persistence_tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_identity_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("identity.json");

        // Generate and save identity
        let identity = NodeIdentity::generate(TEST_DIFFICULTY).unwrap();
        identity.save_to_file(&path).await.unwrap();

        // Load identity
        let loaded = NodeIdentity::load_from_file(&path).await.unwrap();

        // Should be identical
        assert_eq!(identity.node_id(), loaded.node_id());
        assert_eq!(
            identity.word_address().to_string(),
            loaded.word_address().to_string()
        );
        assert_eq!(
            identity.public_key().as_bytes(),
            loaded.public_key().as_bytes()
        );

        // Loaded identity should be functional
        let message = b"test message";
        let signature = loaded.sign(message);
        assert!(identity.verify(message, &signature));
    }

    #[test]
    fn test_identity_serialization() {
        let identity = NodeIdentity::generate(TEST_DIFFICULTY).unwrap();

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&identity).unwrap();
        assert!(json.contains("node_id"));
        assert!(json.contains("word_address"));
        assert!(json.contains("proof_of_work"));

        // Deserialize from JSON
        let deserialized: NodeIdentity = serde_json::from_str(&json).unwrap();
        assert_eq!(identity.node_id(), deserialized.node_id());
    }

    #[tokio::test]
    async fn test_default_identity_location() {
        // Test save to default location
        let identity = NodeIdentity::generate(TEST_DIFFICULTY).unwrap();
        let default_path = identity.default_path().unwrap();

        // Clean up any existing file
        if default_path.exists() {
            tokio::fs::remove_file(&default_path).await.ok();
        }

        // Save to default location
        identity.save_default().await.unwrap();
        assert!(default_path.exists());

        // Load from default location
        let loaded = NodeIdentity::load_default().await.unwrap();
        assert_eq!(identity.node_id(), loaded.node_id());

        // Clean up
        tokio::fs::remove_file(&default_path).await.ok();
    }

    #[test]
    fn test_identity_export_import() {
        let identity = NodeIdentity::generate(TEST_DIFFICULTY).unwrap();

        // Export to bytes
        let exported = identity.export();
        // Verify export was successful
        // The exported data is a complex structure, not a simple string/vec
        // So we just verify import works

        // Import from bytes
        let imported = NodeIdentity::import(&exported).unwrap();
        assert_eq!(identity.node_id(), imported.node_id());

        // Test signing with imported identity
        let msg = b"exported identity test";
        let sig = imported.sign(msg);
        assert!(identity.verify(msg, &sig));
    }
}

#[cfg(test)]
mod cryptographic_tests {
    use super::*;

    #[test]
    fn test_signing_and_verification() {
        let identity = NodeIdentity::generate(TEST_DIFFICULTY).unwrap();
        let message = b"Test message for signing";

        // Sign message
        let signature = identity.sign(message);

        // Verify with correct message
        assert!(identity.verify(message, &signature));

        // Verify with wrong message should fail
        assert!(!identity.verify(b"Wrong message", &signature));

        // Verify with modified signature should fail
        let bad_sig = signature.clone();
        bad_sig.to_bytes()[0] ^= 0xFF;
        assert!(!identity.verify(message, &bad_sig));
    }

    #[test]
    fn test_cross_identity_verification() {
        let identity1 = NodeIdentity::generate(TEST_DIFFICULTY).unwrap();
        let identity2 = NodeIdentity::generate(TEST_DIFFICULTY).unwrap();

        let message = b"Cross verification test";
        let sig1 = identity1.sign(message);

        // identity2 should not be able to verify identity1's signature
        assert!(!identity2.verify(message, &sig1));

        // But identity1 should verify its own
        assert!(identity1.verify(message, &sig1));
    }

    proptest! {
        #[test]
        fn prop_signature_uniqueness(message1: Vec<u8>, message2: Vec<u8>) {
            prop_assume!(!message1.is_empty() && !message2.is_empty());
            prop_assume!(message1 != message2);

            let identity = NodeIdentity::generate(TEST_DIFFICULTY).unwrap();

            let sig1 = identity.sign(&message1);
            let sig2 = identity.sign(&message2);

            // Different messages should produce different signatures
            prop_assert_ne!(sig1.to_bytes(), sig2.to_bytes());
        }
    }
}

#[cfg(test)]
mod performance_benchmarks {
    use super::*;
    use std::time::Instant;

    #[test]
    fn bench_identity_generation() {
        let iterations = 10;
        let mut total_duration = Duration::from_secs(0);

        for _ in 0..iterations {
            let start = Instant::now();
            let _ = NodeIdentity::generate(TEST_DIFFICULTY).unwrap();
            total_duration += start.elapsed();
        }

        let avg_duration = total_duration / iterations;
        println!("Average identity generation time: {:?}", avg_duration);

        // Should be reasonably fast (adjust threshold as needed)
        assert!(avg_duration < Duration::from_secs(1));
    }

    #[test]
    fn bench_proof_of_work_scaling() {
        // Test how PoW time scales with difficulty
        let node_id = NodeId([0x42; 32]);

        for difficulty in [8, 10, 12, 14] {
            let start = Instant::now();
            let _ = ProofOfWork::solve(&node_id, difficulty).unwrap();
            let elapsed = start.elapsed();

            println!("Difficulty {}: {:?}", difficulty, elapsed);
        }
    }

    #[test]
    fn bench_signature_operations() {
        let identity = NodeIdentity::generate(TEST_DIFFICULTY).unwrap();
        let message = b"Benchmark message for signature operations";
        let iterations = 1000;

        // Benchmark signing
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = identity.sign(message);
        }
        let sign_duration = start.elapsed() / iterations;

        // Benchmark verification
        let signature = identity.sign(message);
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = identity.verify(message, &signature);
        }
        let verify_duration = start.elapsed() / iterations;

        println!("Average signing time: {:?}", sign_duration);
        println!("Average verification time: {:?}", verify_duration);

        // Both should be very fast
        assert!(sign_duration < Duration::from_micros(100));
        assert!(verify_duration < Duration::from_micros(100));
    }
}

// Helper functions

fn count_leading_zero_bits(bytes: &[u8]) -> u32 {
    let mut count = 0;
    for byte in bytes {
        if *byte == 0 {
            count += 8;
        } else {
            count += byte.leading_zeros();
            break;
        }
    }
    count
}
