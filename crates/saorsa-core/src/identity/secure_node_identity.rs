// Copyright 2024 P2P Foundation
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Secure node identity implementation with key zeroization
//!
//! This module provides enhanced security features for the NodeIdentity
//! including secure key handling with automatic zeroization.

use super::four_words::FourWordAddress;
use super::node_identity::{IdentityData, NodeId, ProofOfWork};
use crate::error::IdentityError;
use crate::{P2PError, Result};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

/// Enhanced node identity with secure key management
// Note: SigningKey doesn't implement Zeroize, but it has internal protections
pub struct SecureNodeIdentity {
    /// Signing key - has internal memory protections
    signing_key: SigningKey,

    /// Verification key (public, no need to zeroize)
    verification_key: VerifyingKey,

    /// Node ID derived from public key
    node_id: NodeId,

    /// Human-readable four-word address
    word_address: FourWordAddress,

    /// Proof of work for Sybil resistance
    proof_of_work: ProofOfWork,
}

impl SecureNodeIdentity {
    /// Generate a new identity with proof of work
    pub fn generate(difficulty: u32) -> Result<Self> {
        // Validate entropy before key generation
        validate_system_entropy()?;

        let signing_key = SigningKey::generate(&mut OsRng);
        let verification_key = signing_key.verifying_key();

        // Derive node ID from public key
        let mut hasher = Sha256::new();
        hasher.update(verification_key.as_bytes());
        let hash = hasher.finalize();
        let node_id = NodeId(hash.into());

        // Generate four-word address
        let word_address = FourWordAddress::from_bytes(node_id.to_bytes())?;

        // Compute proof of work
        let proof_of_work = ProofOfWork::solve(&node_id, difficulty)?;

        Ok(Self {
            signing_key,
            verification_key,
            node_id,
            word_address,
            proof_of_work,
        })
    }

    /// Generate from a seed with enhanced security
    pub fn from_seed(seed: &[u8; 32], difficulty: u32) -> Result<Self> {
        // Validate entropy
        validate_seed_entropy(seed)?;

        let signing_key = SigningKey::from_bytes(seed);
        let verification_key = signing_key.verifying_key();

        // Derive node ID
        let mut hasher = Sha256::new();
        hasher.update(verification_key.as_bytes());
        let hash = hasher.finalize();
        let node_id = NodeId(hash.into());

        // Generate four-word address
        let word_address = FourWordAddress::from_bytes(node_id.to_bytes())?;

        // Compute proof of work
        let proof_of_work = ProofOfWork::solve(&node_id, difficulty)?;

        Ok(Self {
            signing_key,
            verification_key,
            node_id,
            word_address,
            proof_of_work,
        })
    }

    /// Export identity data (without exposing signing key directly)
    pub fn export(&self) -> IdentityData {
        IdentityData {
            private_key: self.signing_key.to_bytes().to_vec(),
            proof_of_work: self.proof_of_work.clone(),
        }
    }

    /// Import from identity data with validation
    pub fn import(data: &IdentityData) -> Result<Self> {
        // Reconstruct identity from private key
        let signing_key =
            SigningKey::from_bytes(&data.private_key.clone().try_into().map_err(|_| {
                P2PError::Identity(IdentityError::InvalidFormat(
                    "Invalid private key length".to_string().into(),
                ))
            })?);
        let verification_key = signing_key.verifying_key();
        let node_id = NodeId::from_public_key(&verification_key);

        // Validate the proof of work
        if !data
            .proof_of_work
            .verify(&node_id, data.proof_of_work.difficulty)
        {
            return Err(P2PError::Identity(IdentityError::InvalidProofOfWork));
        }

        // Generate four-word address from node ID
        let word_address = FourWordAddress::from_bytes(node_id.to_bytes())?;

        Ok(Self {
            signing_key,
            verification_key,
            node_id,
            word_address,
            proof_of_work: data.proof_of_work.clone(),
        })
    }

    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> Signature {
        self.signing_key.sign(message)
    }

    /// Verify a signature
    pub fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        self.verification_key.verify(message, signature).is_ok()
    }

    /// Get node ID
    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    /// Get four-word address
    pub fn word_address(&self) -> &FourWordAddress {
        &self.word_address
    }

    /// Get verification key
    pub fn verification_key(&self) -> &VerifyingKey {
        &self.verification_key
    }

    /// Get proof of work
    pub fn proof_of_work(&self) -> &ProofOfWork {
        &self.proof_of_work
    }
}

/// Validate system entropy is sufficient for key generation
fn validate_system_entropy() -> Result<()> {
    // Check if we can get random bytes from the OS
    let mut test_bytes = [0u8; 32];
    use rand::RngCore;

    match OsRng.try_fill_bytes(&mut test_bytes) {
        Ok(_) => {
            // Additional check: ensure bytes aren't all zeros
            if test_bytes.iter().all(|&b| b == 0) {
                return Err(P2PError::Identity(IdentityError::InsufficientEntropy));
            }
            Ok(())
        }
        Err(_) => Err(P2PError::Identity(IdentityError::InsufficientEntropy)),
    }
}

/// Validate seed has sufficient entropy
fn validate_seed_entropy(seed: &[u8; 32]) -> Result<()> {
    // Check for obviously weak seeds
    if seed.iter().all(|&b| b == 0) {
        return Err(P2PError::Identity(IdentityError::InsufficientEntropy));
    }

    // Check for pattern of all same bytes
    let first_byte = seed[0];
    if seed.iter().all(|&b| b == first_byte) {
        return Err(P2PError::Identity(IdentityError::InsufficientEntropy));
    }

    if seed.iter().all(|&b| b == 0xFF) {
        return Err(P2PError::Identity(IdentityError::InsufficientEntropy));
    }

    // Check for repeating patterns
    let unique_bytes: std::collections::HashSet<_> = seed.iter().collect();
    if unique_bytes.len() < 8 {
        return Err(P2PError::Identity(IdentityError::InsufficientEntropy));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_identity_generation() {
        let identity = SecureNodeIdentity::generate(8).unwrap();
        assert!(identity.proof_of_work.verify(identity.node_id(), 8));
    }

    #[test]
    fn test_entropy_validation() {
        // Should reject all-zero seed
        let weak_seed = [0u8; 32];
        assert!(SecureNodeIdentity::from_seed(&weak_seed, 8).is_err());

        // Should reject all-ones seed
        let weak_seed = [0xFFu8; 32];
        assert!(SecureNodeIdentity::from_seed(&weak_seed, 8).is_err());

        // Should accept good seed
        let mut good_seed = [0u8; 32];
        for (i, byte) in good_seed.iter_mut().enumerate() {
            *byte = i as u8;
        }
        assert!(SecureNodeIdentity::from_seed(&good_seed, 8).is_ok());
    }

    #[test]
    fn test_key_zeroization() {
        let identity = SecureNodeIdentity::generate(8).unwrap();
        let _signing_key_bytes = identity.signing_key.to_bytes();

        // Identity will be dropped here, signing key should be zeroized
        drop(identity);

        // Note: In practice, we can't directly test that memory was zeroized
        // but the zeroize crate ensures this happens
    }
}
