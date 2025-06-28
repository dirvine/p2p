//! ML-KEM (Module-Lattice Key Encapsulation Mechanism) implementation
//! 
//! Implements FIPS 203 standard for quantum-resistant key exchange

use super::{QuantumCryptoError, Result};
use crate::quantum_crypto::types::*;
use ml_kem::{MlKem768, EncapsulatePair, DecapsulatePair};
use rand::rngs::OsRng;

/// ML-KEM-768 parameter set (recommended by NIST)
pub type MlKem = MlKem768;

/// Generate ML-KEM keypair
pub fn generate_keypair() -> Result<(Vec<u8>, Vec<u8>)> {
    let (decapsulation_key, encapsulation_key) = MlKem::generate(&mut OsRng);
    
    // Serialize keys
    let public_key = encapsulation_key.as_bytes().to_vec();
    let private_key = decapsulation_key.as_bytes().to_vec();
    
    Ok((public_key, private_key))
}

/// Encapsulate a shared secret using ML-KEM public key
pub fn encapsulate(public_key: &[u8]) -> Result<(Vec<u8>, SharedSecret)> {
    // Parse public key
    let encapsulation_key = ml_kem::EncodedSizeUser::from_bytes(public_key.into())
        .map_err(|e| QuantumCryptoError::MlKemError(format!("Invalid public key: {}", e)))?;
    
    // Generate shared secret and ciphertext
    let EncapsulatePair { ciphertext, shared_secret } = 
        encapsulation_key.encapsulate(&mut OsRng)
            .map_err(|e| QuantumCryptoError::EncapsulationError(e.to_string()))?;
    
    let ciphertext_bytes = ciphertext.as_bytes().to_vec();
    let shared_secret = SharedSecret(shared_secret.into());
    
    Ok((ciphertext_bytes, shared_secret))
}

/// Decapsulate shared secret using ML-KEM private key
pub fn decapsulate(private_key: &[u8], ciphertext: &[u8]) -> Result<SharedSecret> {
    // Parse private key
    let decapsulation_key = ml_kem::EncodedSizeUser::from_bytes(private_key.into())
        .map_err(|e| QuantumCryptoError::MlKemError(format!("Invalid private key: {}", e)))?;
    
    // Parse ciphertext
    let ciphertext_obj = ml_kem::EncodedSizeUser::from_bytes(ciphertext.into())
        .map_err(|e| QuantumCryptoError::MlKemError(format!("Invalid ciphertext: {}", e)))?;
    
    // Decapsulate shared secret
    let DecapsulatePair { shared_secret } = 
        decapsulation_key.decapsulate(&ciphertext_obj)
            .map_err(|e| QuantumCryptoError::DecapsulationError(e.to_string()))?;
    
    Ok(SharedSecret(shared_secret.into()))
}

/// ML-KEM key exchange state for handshake protocol
pub struct MlKemState {
    /// Our keypair
    pub keypair: Option<(MlKemPublicKey, MlKemPrivateKey)>,
    
    /// Remote public key
    pub remote_public_key: Option<MlKemPublicKey>,
    
    /// Shared secret (after exchange)
    pub shared_secret: Option<SharedSecret>,
    
    /// Role in the exchange
    pub role: KeyExchangeRole,
}

/// Role in key exchange
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyExchangeRole {
    Initiator,
    Responder,
}

impl MlKemState {
    /// Create new ML-KEM state
    pub fn new(role: KeyExchangeRole) -> Self {
        Self {
            keypair: None,
            remote_public_key: None,
            shared_secret: None,
            role,
        }
    }
    
    /// Generate our keypair
    pub fn generate_keypair(&mut self) -> Result<MlKemPublicKey> {
        let (public_key, private_key) = generate_keypair()?;
        
        let public = MlKemPublicKey(public_key);
        let private = MlKemPrivateKey(private_key);
        
        self.keypair = Some((public.clone(), private));
        
        Ok(public)
    }
    
    /// Set remote public key
    pub fn set_remote_public_key(&mut self, public_key: MlKemPublicKey) {
        self.remote_public_key = Some(public_key);
    }
    
    /// Complete key exchange as initiator
    pub fn complete_as_initiator(&mut self, ciphertext: &MlKemCiphertext) -> Result<SharedSecret> {
        match self.role {
            KeyExchangeRole::Initiator => {
                let (_, private_key) = self.keypair.as_ref()
                    .ok_or_else(|| QuantumCryptoError::InvalidKeyError(
                        "No local keypair generated".to_string()
                    ))?;
                
                let shared_secret = decapsulate(&private_key.0, &ciphertext.0)?;
                self.shared_secret = Some(shared_secret.clone());
                
                Ok(shared_secret)
            }
            KeyExchangeRole::Responder => {
                Err(QuantumCryptoError::InvalidKeyError(
                    "Cannot complete as initiator when role is responder".to_string()
                ))
            }
        }
    }
    
    /// Complete key exchange as responder
    pub fn complete_as_responder(&mut self) -> Result<(MlKemCiphertext, SharedSecret)> {
        match self.role {
            KeyExchangeRole::Responder => {
                let remote_key = self.remote_public_key.as_ref()
                    .ok_or_else(|| QuantumCryptoError::InvalidKeyError(
                        "No remote public key set".to_string()
                    ))?;
                
                let (ciphertext, shared_secret) = encapsulate(&remote_key.0)?;
                self.shared_secret = Some(shared_secret.clone());
                
                Ok((MlKemCiphertext(ciphertext), shared_secret))
            }
            KeyExchangeRole::Initiator => {
                Err(QuantumCryptoError::InvalidKeyError(
                    "Cannot complete as responder when role is initiator".to_string()
                ))
            }
        }
    }
}

/// Hybrid key exchange combining ML-KEM with classical ECDH
pub struct HybridKeyExchange {
    pub ml_kem_state: MlKemState,
    pub classical_shared: Option<[u8; 32]>,
}

impl HybridKeyExchange {
    /// Create new hybrid key exchange
    pub fn new(role: KeyExchangeRole) -> Self {
        Self {
            ml_kem_state: MlKemState::new(role),
            classical_shared: None,
        }
    }
    
    /// Combine ML-KEM and classical shared secrets
    pub fn derive_hybrid_secret(&self) -> Result<[u8; 32]> {
        let ml_kem_secret = self.ml_kem_state.shared_secret.as_ref()
            .ok_or_else(|| QuantumCryptoError::InvalidKeyError(
                "ML-KEM exchange not complete".to_string()
            ))?;
        
        let classical_secret = self.classical_shared.as_ref()
            .ok_or_else(|| QuantumCryptoError::InvalidKeyError(
                "Classical exchange not complete".to_string()
            ))?;
        
        // Combine using HKDF
        use hkdf::Hkdf;
        use sha2::Sha256;
        
        let mut combined = Vec::new();
        combined.extend_from_slice(ml_kem_secret.as_bytes());
        combined.extend_from_slice(classical_secret);
        
        let hkdf = Hkdf::<Sha256>::new(None, &combined);
        let mut output = [0u8; 32];
        hkdf.expand(b"hybrid-key-exchange", &mut output)
            .map_err(|e| QuantumCryptoError::MlKemError(format!("HKDF failed: {}", e)))?;
        
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ml_kem_key_exchange() {
        // Alice (initiator) generates keypair
        let mut alice_state = MlKemState::new(KeyExchangeRole::Initiator);
        let alice_public = alice_state.generate_keypair().unwrap();
        
        // Bob (responder) receives Alice's public key
        let mut bob_state = MlKemState::new(KeyExchangeRole::Responder);
        bob_state.set_remote_public_key(alice_public);
        
        // Bob encapsulates shared secret
        let (ciphertext, bob_secret) = bob_state.complete_as_responder().unwrap();
        
        // Alice decapsulates shared secret
        let alice_secret = alice_state.complete_as_initiator(&ciphertext).unwrap();
        
        // Secrets should match
        assert_eq!(alice_secret.as_bytes(), bob_secret.as_bytes());
    }
    
    #[test]
    fn test_keypair_generation() {
        let (public_key, private_key) = generate_keypair().unwrap();
        
        // Check key sizes (ML-KEM-768)
        assert_eq!(public_key.len(), 1184);
        assert_eq!(private_key.len(), 2400);
    }
    
    #[test]
    fn test_invalid_role_operations() {
        let mut initiator = MlKemState::new(KeyExchangeRole::Initiator);
        let mut responder = MlKemState::new(KeyExchangeRole::Responder);
        
        // Initiator cannot complete as responder
        assert!(initiator.complete_as_responder().is_err());
        
        // Responder cannot complete as initiator
        let dummy_ciphertext = MlKemCiphertext(vec![0; 1088]);
        assert!(responder.complete_as_initiator(&dummy_ciphertext).is_err());
    }
}