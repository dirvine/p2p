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

//! Core types for quantum-resistant cryptography

use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::SystemTime;

/// Peer identifier derived from quantum-resistant public key
///
/// Unique identifier for peers in the quantum-resistant P2P network.
/// Generated from a cryptographic hash of the peer's ML-DSA public key
/// to ensure uniqueness and prevent spoofing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId(pub Vec<u8>);

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.0[..8]))
    }
}

/// Unique identifier for threshold cryptography groups
///
/// 256-bit identifier for groups participating in threshold signature
/// schemes, distributed key generation, and quantum-resistant consensus.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GroupId(pub [u8; 32]);

impl fmt::Display for GroupId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.0[..8]))
    }
}

/// Participant identifier within threshold cryptography groups
///
/// Numeric identifier for individual participants in threshold schemes.
/// Limited to u16 range to support groups up to 65,535 participants.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ParticipantId(pub u16);

impl std::fmt::Display for ParticipantId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Session identifier for cryptographic operations
///
/// 256-bit identifier for temporary cryptographic sessions including
/// key exchange, signature ceremonies, and secure communications.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub [u8; 32]);

/// Complete quantum-resistant peer identity
///
/// Contains all cryptographic material needed for secure quantum-resistant
/// communication including post-quantum signatures and key exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumPeerIdentity {
    /// Unique identifier for the peer
    pub peer_id: PeerId,

    /// ML-DSA (FIPS 204) public key for post-quantum digital signatures
    pub ml_dsa_public_key: MlDsaPublicKey,

    /// ML-KEM (FIPS 203) public key for quantum-safe key exchange
    pub ml_kem_public_key: MlKemPublicKey,

    /// Optional FROST public key for threshold operations
    pub frost_public_key: Option<FrostPublicKey>,

    /// Classical Ed25519 key for backward compatibility
    pub legacy_key: Option<Ed25519PublicKey>,

    /// Supported cryptographic capabilities
    pub capabilities: crate::quantum_crypto::CryptoCapabilities,

    /// Identity creation timestamp
    pub created_at: SystemTime,
}

/// ML-DSA public key wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlDsaPublicKey(pub Vec<u8>);

/// ML-DSA private key wrapper
#[derive(Clone)]
pub struct MlDsaPrivateKey(pub Vec<u8>);

impl fmt::Debug for MlDsaPrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("MlDsaPrivateKey").field(&"***").finish()
    }
}

/// ML-KEM public key wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlKemPublicKey(pub Vec<u8>);

/// ML-KEM private key wrapper
#[derive(Clone)]
pub struct MlKemPrivateKey(pub Vec<u8>);

impl fmt::Debug for MlKemPrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("MlKemPrivateKey").field(&"***").finish()
    }
}

/// FROST public key for threshold signatures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrostPublicKey(pub Vec<u8>);

/// FROST group public key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrostGroupPublicKey(pub Vec<u8>);

/// FROST key share for a participant
#[derive(Clone)]
pub struct FrostKeyShare {
    pub participant_id: ParticipantId,
    pub share: Vec<u8>,
}

impl fmt::Debug for FrostKeyShare {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FrostKeyShare")
            .field("participant_id", &self.participant_id)
            .field("share", &"***")
            .finish()
    }
}

/// FROST commitment for verifiable secret sharing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrostCommitment(pub Vec<u8>);

/// Ed25519 public key for backward compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ed25519PublicKey(pub [u8; 32]);

/// Ed25519 private key
#[derive(Clone, Serialize, Deserialize)]
pub struct Ed25519PrivateKey(#[serde(with = "serde_big_array::BigArray")] pub [u8; 64]);

impl fmt::Debug for Ed25519PrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Ed25519PrivateKey").field(&"***").finish()
    }
}

/// ML-DSA signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlDsaSignature(pub Vec<u8>);

/// FROST signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrostSignature(pub Vec<u8>);

/// Ed25519 signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ed25519Signature(#[serde(with = "serde_big_array::BigArray")] pub [u8; 64]);

/// Combined signature for hybrid mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSignature {
    pub classical: Ed25519Signature,
    pub post_quantum: MlDsaSignature,
}

/// ML-KEM ciphertext
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlKemCiphertext(pub Vec<u8>);

/// Shared secret derived from KEM
#[derive(Clone)]
pub struct SharedSecret(pub [u8; 32]);

impl fmt::Debug for SharedSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SharedSecret").field(&"***").finish()
    }
}

impl SharedSecret {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Quantum-safe secure session
#[derive(Debug)]
pub struct SecureSession {
    /// Session identifier
    pub session_id: SessionId,

    /// Symmetric encryption key (derived from ML-KEM)
    pub encryption_key: [u8; 32],

    /// Message authentication key
    pub mac_key: [u8; 32],

    /// Remote peer identity
    pub peer_identity: QuantumPeerIdentity,

    /// Session establishment time
    pub established_at: SystemTime,

    /// Session state
    pub state: SessionState,

    /// Whether this is a threshold-capable session
    pub is_threshold_capable: bool,
}

/// Session state
#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    /// Handshake in progress
    Handshaking,

    /// Session established and active
    Active,

    /// Session closed
    Closed,
}

/// Handshake parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeParameters {
    pub kem_algorithm: crate::quantum_crypto::KemAlgorithm,
    pub signature_algorithm: crate::quantum_crypto::SignatureAlgorithm,
    pub hybrid_mode: bool,
    pub protocol_version: crate::quantum_crypto::ProtocolVersion,
}

/// Key derivation info
pub struct KeyDerivationInfo {
    pub purpose: KeyPurpose,
    pub session_id: SessionId,
    pub additional_data: Vec<u8>,
}

/// Key purpose for derivation
#[derive(Debug, Clone, Copy)]
pub enum KeyPurpose {
    Encryption,
    Authentication,
    KeyWrapping,
}

/// Set of public keys for hybrid cryptography
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicKeySet {
    /// Ed25519 public key for classical signatures
    pub ed25519: Option<Ed25519PublicKey>,
    /// ML-DSA public key for post-quantum signatures
    pub ml_dsa: Option<MlDsaPublicKey>,
    /// ML-KEM public key for quantum-safe key exchange
    pub ml_kem: Option<MlKemPublicKey>,
    /// FROST public key for threshold operations
    pub frost: Option<FrostPublicKey>,
}

/// Set of private keys for hybrid cryptography
pub struct PrivateKeySet {
    /// Ed25519 private key for classical signatures
    pub ed25519: Option<Ed25519PrivateKey>,
    /// ML-DSA private key for post-quantum signatures
    pub ml_dsa: Option<MlDsaPrivateKey>,
    /// ML-KEM private key for quantum-safe key exchange
    pub ml_kem: Option<MlKemPrivateKey>,
    /// FROST key share for threshold operations
    pub frost: Option<FrostKeyShare>,
}

impl fmt::Debug for PrivateKeySet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PrivateKeySet")
            .field("ed25519", &self.ed25519.as_ref().map(|_| "***"))
            .field("ml_dsa", &self.ml_dsa.as_ref().map(|_| "***"))
            .field("ml_kem", &self.ml_kem.as_ref().map(|_| "***"))
            .field("frost", &self.frost.as_ref().map(|_| "***"))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_id_display() {
        let peer_id = PeerId(vec![0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0]);
        assert_eq!(format!("{}", peer_id), "123456789abcdef0");
    }

    #[test]
    fn test_sensitive_debug() -> Result<(), Box<dyn std::error::Error>> {
        let private_key = MlDsaPrivateKey(vec![0x42; 32]);
        let debug_str: String = format!("{:?}", private_key);
        assert!(!debug_str.contains("0x42"));
        assert!(debug_str.contains("***"));
        Ok(())
    }
}
