//! Cryptographic identity system for the adaptive P2P network
//! 
//! Implements Ed25519-based identity with proof-of-work puzzles for Sybil resistance
//! as specified in the network design documentation.

use super::*;
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature, Signer, Verifier};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// Difficulty target for proof-of-work
const DEFAULT_POW_DIFFICULTY: u8 = 16; // Number of leading zero bits required

/// Node identity with cryptographic keys and proof-of-work
pub struct NodeIdentity {
    /// Ed25519 keypair
    keypair: Keypair,
    /// Node ID derived from public key
    node_id: NodeId,
    /// Proof of work for Sybil resistance
    proof_of_work: ProofOfWork,
}

/// Proof of work structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofOfWork {
    /// Nonce value that satisfies the difficulty
    pub nonce: u64,
    /// Difficulty level (number of leading zeros)
    pub difficulty: u8,
    /// Timestamp when PoW was computed
    pub timestamp: u64,
}

/// Signed message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedMessage<T: Serialize> {
    /// Message payload
    pub payload: T,
    /// Sender's node ID
    pub sender_id: NodeId,
    /// Unix timestamp
    pub timestamp: u64,
    /// Ed25519 signature
    pub signature: Vec<u8>,
}

impl NodeIdentity {
    /// Generate a new node identity with proof-of-work
    pub fn generate() -> Result<Self> {
        let mut csprng = rand::thread_rng();
        let keypair = Keypair::generate(&mut csprng);
        
        let node_id = Self::compute_node_id(&keypair.public);
        let proof_of_work = Self::solve_pow_puzzle(&node_id, DEFAULT_POW_DIFFICULTY)?;
        
        Ok(Self {
            keypair,
            node_id,
            proof_of_work,
        })
    }
    
    /// Create identity from existing keypair (requires proof-of-work)
    pub fn from_keypair(keypair: Keypair) -> Result<Self> {
        let node_id = Self::compute_node_id(&keypair.public);
        let proof_of_work = Self::solve_pow_puzzle(&node_id, DEFAULT_POW_DIFFICULTY)?;
        
        Ok(Self {
            keypair,
            node_id,
            proof_of_work,
        })
    }
    
    /// Compute node ID from public key (SHA-256 hash)
    pub fn compute_node_id(public_key: &PublicKey) -> NodeId {
        let mut hasher = Sha256::new();
        hasher.update(public_key.as_bytes());
        let result = hasher.finalize();
        
        // Convert hash to UserId
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&result);
        crate::peer_record::UserId::from_bytes(bytes)
    }
    
    /// Solve proof-of-work puzzle for given node ID
    pub fn solve_pow_puzzle(node_id: &NodeId, difficulty: u8) -> Result<ProofOfWork> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| AdaptiveNetworkError::Other(e.to_string()))?
            .as_secs();
        
        let mut nonce = 0u64;
        let target_zeros = difficulty / 8; // Full zero bytes
        let remaining_bits = difficulty % 8; // Remaining bits in partial byte
        
        loop {
            let mut hasher = Sha256::new();
            hasher.update(&node_id.hash);
            hasher.update(&nonce.to_le_bytes());
            hasher.update(&timestamp.to_le_bytes());
            
            let result = hasher.finalize();
            
            // Check if we have enough leading zeros
            let mut valid = true;
            
            // Check full zero bytes
            for i in 0..target_zeros {
                if result[i as usize] != 0 {
                    valid = false;
                    break;
                }
            }
            
            // Check remaining bits if needed
            if valid && remaining_bits > 0 {
                let byte_idx = target_zeros as usize;
                let mask = 0xFF << (8 - remaining_bits);
                if (result[byte_idx] & mask) != 0 {
                    valid = false;
                }
            }
            
            if valid {
                return Ok(ProofOfWork {
                    nonce,
                    difficulty,
                    timestamp,
                });
            }
            
            nonce += 1;
            
            // Prevent infinite loops
            if nonce > u64::MAX / 2 {
                return Err(AdaptiveNetworkError::Other(
                    "Failed to solve PoW puzzle after maximum attempts".to_string()
                ));
            }
        }
    }
    
    /// Verify proof-of-work for a node ID
    pub fn verify_pow(node_id: &NodeId, pow: &ProofOfWork) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(&node_id.hash);
        hasher.update(&pow.nonce.to_le_bytes());
        hasher.update(&pow.timestamp.to_le_bytes());
        
        let result = hasher.finalize();
        
        let target_zeros = pow.difficulty / 8;
        let remaining_bits = pow.difficulty % 8;
        
        // Check full zero bytes
        for i in 0..target_zeros {
            if result[i as usize] != 0 {
                return false;
            }
        }
        
        // Check remaining bits
        if remaining_bits > 0 {
            let byte_idx = target_zeros as usize;
            let mask = 0xFF << (8 - remaining_bits);
            if (result[byte_idx] & mask) != 0 {
                return false;
            }
        }
        
        true
    }
    
    /// Sign a message
    pub fn sign_message<T: Serialize + Clone>(&self, message: &T) -> Result<SignedMessage<T>> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| AdaptiveNetworkError::Other(e.to_string()))?
            .as_secs();
        
        let payload_bytes = bincode::serialize(message)
            .map_err(AdaptiveNetworkError::Serialization)?;
        
        // Create bytes to sign: payload || sender_id || timestamp
        let mut bytes_to_sign = Vec::new();
        bytes_to_sign.extend_from_slice(&payload_bytes);
        bytes_to_sign.extend_from_slice(&self.node_id.hash);
        bytes_to_sign.extend_from_slice(&timestamp.to_le_bytes());
        
        let signature = self.keypair.sign(&bytes_to_sign);
        
        Ok(SignedMessage {
            payload: message.clone(),
            sender_id: self.node_id.clone(),
            timestamp,
            signature: signature.to_bytes().to_vec(),
        })
    }
    
    /// Get node ID
    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }
    
    /// Get public key
    pub fn public_key(&self) -> &PublicKey {
        &self.keypair.public
    }
    
    /// Get proof of work
    pub fn proof_of_work(&self) -> &ProofOfWork {
        &self.proof_of_work
    }
}

impl<T: Serialize + for<'de> Deserialize<'de>> SignedMessage<T> {
    /// Verify message signature
    pub fn verify(&self, public_key: &PublicKey) -> Result<bool> {
        let payload_bytes = bincode::serialize(&self.payload)
            .map_err(AdaptiveNetworkError::Serialization)?;
        
        // Recreate bytes that were signed
        let mut bytes_to_verify = Vec::new();
        bytes_to_verify.extend_from_slice(&payload_bytes);
        bytes_to_verify.extend_from_slice(&self.sender_id.hash);
        bytes_to_verify.extend_from_slice(&self.timestamp.to_le_bytes());
        
        let signature = Signature::from_bytes(&self.signature)
            .map_err(|e| AdaptiveNetworkError::Other(format!("Invalid signature: {}", e)))?;
        
        Ok(public_key.verify(&bytes_to_verify, &signature).is_ok())
    }
    
    /// Get message age in seconds
    pub fn age(&self) -> Result<u64> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| AdaptiveNetworkError::Other(e.to_string()))?
            .as_secs();
        
        Ok(now.saturating_sub(self.timestamp))
    }
}

/// Identity storage for persistence
#[derive(Debug, Serialize, Deserialize)]
pub struct StoredIdentity {
    /// Secret key bytes
    pub secret_key: Vec<u8>,
    /// Public key bytes
    pub public_key: Vec<u8>,
    /// Node ID
    pub node_id: NodeId,
    /// Proof of work
    pub proof_of_work: ProofOfWork,
}

impl StoredIdentity {
    /// Create from NodeIdentity
    pub fn from_identity(identity: &NodeIdentity) -> Self {
        Self {
            secret_key: identity.keypair.secret.to_bytes().to_vec(),
            public_key: identity.keypair.public.to_bytes().to_vec(),
            node_id: identity.node_id.clone(),
            proof_of_work: identity.proof_of_work.clone(),
        }
    }
    
    /// Restore to NodeIdentity
    pub fn to_identity(&self) -> Result<NodeIdentity> {
        let secret_key = SecretKey::from_bytes(&self.secret_key)
            .map_err(|e| AdaptiveNetworkError::Other(format!("Invalid secret key: {}", e)))?;
        
        let public_key = PublicKey::from_bytes(&self.public_key)
            .map_err(|e| AdaptiveNetworkError::Other(format!("Invalid public key: {}", e)))?;
        
        let keypair = Keypair {
            secret: secret_key,
            public: public_key,
        };
        
        // Verify the stored node ID matches
        let computed_id = NodeIdentity::compute_node_id(&public_key);
        if computed_id != self.node_id {
            return Err(AdaptiveNetworkError::Other(
                "Stored node ID doesn't match computed ID".to_string()
            ));
        }
        
        // Verify proof of work
        if !NodeIdentity::verify_pow(&self.node_id, &self.proof_of_work) {
            return Err(AdaptiveNetworkError::Other(
                "Invalid proof of work".to_string()
            ));
        }
        
        Ok(NodeIdentity {
            keypair,
            node_id: self.node_id.clone(),
            proof_of_work: self.proof_of_work.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_identity_generation() {
        let identity = NodeIdentity::generate().unwrap();
        
        // Verify node ID matches public key
        let computed_id = NodeIdentity::compute_node_id(identity.public_key());
        assert_eq!(&computed_id, identity.node_id());
        
        // Verify proof of work
        assert!(NodeIdentity::verify_pow(
            identity.node_id(),
            identity.proof_of_work()
        ));
    }
    
    #[test]
    fn test_message_signing_and_verification() {
        let identity = NodeIdentity::generate().unwrap();
        
        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
        struct TestMessage {
            content: String,
            value: u64,
        }
        
        let message = TestMessage {
            content: "Hello, P2P!".to_string(),
            value: 42,
        };
        
        // Sign message
        let signed = identity.sign_message(&message).unwrap();
        
        // Verify with correct public key
        assert!(signed.verify(identity.public_key()).unwrap());
        
        // Verify with wrong public key should fail
        let other_identity = NodeIdentity::generate().unwrap();
        assert!(!signed.verify(other_identity.public_key()).unwrap());
    }
    
    #[test]
    fn test_proof_of_work_verification() {
        use crate::peer_record::UserId;
        
        let node_id = UserId::random();
        let pow = NodeIdentity::solve_pow_puzzle(&node_id, 8).unwrap();
        
        assert!(NodeIdentity::verify_pow(&node_id, &pow));
        
        // Modified nonce should fail
        let mut bad_pow = pow.clone();
        bad_pow.nonce += 1;
        assert!(!NodeIdentity::verify_pow(&node_id, &bad_pow));
    }
    
    #[test]
    fn test_identity_serialization() {
        let identity = NodeIdentity::generate().unwrap();
        
        // Store identity
        let stored = StoredIdentity::from_identity(&identity);
        
        // Restore identity
        let restored = stored.to_identity().unwrap();
        
        // Verify they match
        assert_eq!(identity.node_id(), restored.node_id());
        assert_eq!(
            identity.public_key().as_bytes(),
            restored.public_key().as_bytes()
        );
    }
}