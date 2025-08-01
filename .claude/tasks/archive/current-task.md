# Task 1: Core Identity System with Four-Word Addresses

## Task Status
- **Status**: 🟡 In Progress
- **Priority**: Critical
- **Started**: 2025-07-28
- **Assigned**: Claude
- **Estimated**: 3 days

## Context Loaded
- **Specification**: P2P Foundation Specification v4
- **Design**: P2P Foundation Design Document
- **Tech Stack**: Rust, Ed25519, four-word-networking crate, Tokio
- **Standards**: TDD, >80% test coverage, property-based testing

## Acceptance Criteria
- [ ] Implement `NodeIdentity` with Ed25519 keys
- [ ] Integrate four-word-networking crate
- [ ] Generate deterministic four-word addresses from peer IDs
- [ ] Implement proof-of-work for Sybil resistance
- [ ] Create identity persistence and loading
- [ ] Add identity CLI commands

## Tests Required
- Property test: Same seed produces same identity
- Property test: Different seeds produce different addresses
- Unit test: PoW validation
- Integration test: Identity save/load cycle
- Benchmark: Identity generation time

## Implementation Structure
```rust
// Key structure to implement
pub struct NodeIdentity {
    signing_key: SigningKey,
    verification_key: VerifyingKey,
    node_id: NodeId,  // SHA-256(verification_key)
    word_address: FourWordAddress,
    proof_of_work: ProofOfWork,
}
```

## TDD Tests to Write First

### 1. Identity Generation Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_identity_creation() {
        // Test basic identity creation
        let identity = NodeIdentity::generate().unwrap();
        assert!(identity.verify_proof_of_work());
    }

    proptest! {
        #[test]
        fn prop_deterministic_from_seed(seed: [u8; 32]) {
            // Same seed produces same identity
            let id1 = NodeIdentity::from_seed(&seed).unwrap();
            let id2 = NodeIdentity::from_seed(&seed).unwrap();
            assert_eq!(id1.node_id, id2.node_id);
            assert_eq!(id1.word_address, id2.word_address);
        }

        #[test]
        fn prop_different_seeds_different_ids(
            seed1: [u8; 32], 
            seed2: [u8; 32]
        ) {
            // Different seeds produce different identities
            prop_assume!(seed1 != seed2);
            let id1 = NodeIdentity::from_seed(&seed1).unwrap();
            let id2 = NodeIdentity::from_seed(&seed2).unwrap();
            assert_ne!(id1.node_id, id2.node_id);
            assert_ne!(id1.word_address, id2.word_address);
        }
    }
}
```

### 2. Four-Word Address Tests
```rust
#[test]
fn test_four_word_address_generation() {
    let identity = NodeIdentity::generate().unwrap();
    assert_eq!(identity.word_address.words().len(), 4);
    
    // Verify deterministic from node_id
    let addr1 = FourWordAddress::from_node_id(&identity.node_id);
    let addr2 = FourWordAddress::from_node_id(&identity.node_id);
    assert_eq!(addr1, addr2);
}

#[test]
fn test_word_address_parsing() {
    let words = vec!["alpha", "bravo", "charlie", "delta"];
    let addr = FourWordAddress::from_words(&words).unwrap();
    assert_eq!(addr.to_string(), "alpha-bravo-charlie-delta");
}
```

### 3. Proof of Work Tests
```rust
#[test]
fn test_proof_of_work_validation() {
    let identity = NodeIdentity::generate().unwrap();
    assert!(identity.verify_proof_of_work());
    
    // Test invalid PoW
    let mut bad_identity = identity.clone();
    bad_identity.proof_of_work.nonce += 1;
    assert!(!bad_identity.verify_proof_of_work());
}

#[test]
fn test_proof_of_work_difficulty() {
    let pow = ProofOfWork::compute(
        &node_id, 
        MINIMUM_DIFFICULTY
    ).unwrap();
    
    let hash = pow.compute_hash(&node_id);
    let leading_zeros = count_leading_zeros(&hash);
    assert!(leading_zeros >= MINIMUM_DIFFICULTY);
}
```

### 4. Persistence Tests
```rust
#[tokio::test]
async fn test_identity_save_load() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let path = temp_dir.path().join("identity.json");
    
    let identity = NodeIdentity::generate().unwrap();
    identity.save_to_file(&path).await.unwrap();
    
    let loaded = NodeIdentity::load_from_file(&path).await.unwrap();
    assert_eq!(identity.node_id, loaded.node_id);
    assert_eq!(identity.word_address, loaded.word_address);
}

#[test]
fn test_identity_serialization() {
    let identity = NodeIdentity::generate().unwrap();
    let json = serde_json::to_string(&identity).unwrap();
    let deserialized: NodeIdentity = serde_json::from_str(&json).unwrap();
    assert_eq!(identity.node_id, deserialized.node_id);
}
```

### 5. CLI Command Tests
```rust
#[test]
fn test_cli_generate_command() {
    let args = vec!["identity", "generate", "--difficulty", "16"];
    let result = run_cli_command(&args).unwrap();
    assert!(result.contains("word-address"));
    assert!(result.contains("node-id"));
}

#[test]
fn test_cli_show_command() {
    let identity = NodeIdentity::generate().unwrap();
    identity.save_default().unwrap();
    
    let args = vec!["identity", "show"];
    let result = run_cli_command(&args).unwrap();
    assert!(result.contains(&identity.word_address.to_string()));
}
```

## Implementation Steps

### Step 1: Define Core Types
1. Create `NodeIdentity` struct
2. Implement `ProofOfWork` struct
3. Define `FourWordAddress` wrapper

### Step 2: Implement Key Generation
1. Ed25519 key pair generation
2. Node ID calculation (SHA-256 of public key)
3. Deterministic generation from seed

### Step 3: Four-Word Address Integration
1. Integrate four-word-networking crate
2. Map node ID to four words deterministically
3. Implement display and parsing

### Step 4: Proof of Work
1. Implement PoW computation with configurable difficulty
2. Add verification method
3. Benchmark performance

### Step 5: Persistence
1. JSON serialization/deserialization
2. File I/O operations
3. Default location handling

### Step 6: CLI Integration
1. Add identity subcommand
2. Implement generate, show, verify commands
3. Add difficulty configuration

## Notes
- Use existing Ed25519 implementation from crate
- Four-word-networking crate is already in dependencies
- Follow existing error handling patterns
- Ensure thread safety for concurrent identity operations