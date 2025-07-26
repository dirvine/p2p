// Test for new node identity implementation

use saorsa_core::identity::{NodeIdentity, NodeId, ProofOfWork};

#[test]
fn test_node_identity_generation() {
    // Generate with easy difficulty for testing
    let identity = NodeIdentity::generate(8).unwrap();
    
    // Check all fields are set
    assert!(!identity.word_address().is_empty());
    assert!(identity.proof_of_work().verify(identity.node_id(), 8));
    
    println!("Generated identity:");
    println!("  Node ID: {}", identity.node_id());
    println!("  Word address: {}", identity.word_address());
    println!("  PoW computation time: {:?}", identity.proof_of_work().computation_time);
}

#[test]
fn test_deterministic_identity() {
    let seed = [0x42; 32];
    
    // Generate same identity twice
    let id1 = NodeIdentity::from_seed(&seed, 8).unwrap();
    let id2 = NodeIdentity::from_seed(&seed, 8).unwrap();
    
    // Should be identical
    assert_eq!(id1.node_id(), id2.node_id());
    assert_eq!(id1.word_address(), id2.word_address());
}

#[test]
fn test_signing_and_verification() {
    let identity = NodeIdentity::generate(8).unwrap();
    let message = b"Test message for P2P network";
    
    // Sign message
    let signature = identity.sign(message);
    
    // Verify with same identity
    assert!(identity.verify(message, &signature));
    
    // Verify with wrong message should fail
    assert!(!identity.verify(b"Wrong message", &signature));
}

#[test]
fn test_persistence() {
    let identity = NodeIdentity::generate(8).unwrap();
    let original_id = identity.node_id().clone();
    
    // Export to data
    let data = identity.export();
    
    // Import from data
    let restored = NodeIdentity::import(&data).unwrap();
    
    // Should be identical
    assert_eq!(restored.node_id(), &original_id);
    
    // Should be able to sign with restored identity
    let msg = b"Persistence test";
    let sig = restored.sign(msg);
    assert!(identity.verify(msg, &sig));
}

#[test]
fn test_node_id_xor_distance() {
    let id1 = NodeId([0xFF; 32]);
    let id2 = NodeId([0x00; 32]);
    
    let distance = id1.xor_distance(&id2);
    
    // Distance should be all 0xFF
    for byte in distance.iter() {
        assert_eq!(*byte, 0xFF);
    }
}