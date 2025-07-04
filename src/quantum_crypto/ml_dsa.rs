
//! ML-DSA (Module-Lattice Digital Signature Algorithm) implementation
//! 
//! Implements FIPS 204 standard for quantum-resistant digital signatures

use super::{QuantumCryptoError, Result};
use crate::quantum_crypto::types::*;
use ml_dsa::{MlDsa65, SigningKey, VerificationKey, Signature};
use rand::rngs::OsRng;

/// ML-DSA-65 parameter set (recommended by NIST)
pub type MlDsa = MlDsa65;

/// Generate ML-DSA keypair
pub fn generate_keypair() -> Result<(Vec<u8>, Vec<u8>)> {
    let signing_key = SigningKey::<MlDsa>::generate(&mut OsRng);
    let verification_key = signing_key.verification_key();
    
    // Serialize keys
    let public_key = verification_key.to_bytes().to_vec();
    let private_key = signing_key.to_bytes().to_vec();
    
    Ok((public_key, private_key))
}

/// Sign a message with ML-DSA private key
pub fn sign(private_key: &[u8], message: &[u8]) -> Result<Vec<u8>> {
    // Parse private key
    let signing_key = SigningKey::<MlDsa>::from_bytes(private_key.into())
        .map_err(|e| QuantumCryptoError::MlDsaError(format!("Invalid private key: {}", e)))?;
    
    // Sign message
    let signature = signing_key.sign(message);
    
    Ok(signature.to_bytes().to_vec())
}

/// Verify ML-DSA signature
pub fn verify(public_key: &[u8], message: &[u8], signature: &[u8]) -> Result<()> {
    // Parse public key
    let verification_key = VerificationKey::<MlDsa>::from_bytes(public_key.into())
        .map_err(|e| QuantumCryptoError::MlDsaError(format!("Invalid public key: {}", e)))?;
    
    // Parse signature
    let signature_obj = Signature::from_bytes(signature.into())
        .map_err(|e| QuantumCryptoError::MlDsaError(format!("Invalid signature: {}", e)))?;
    
    // Verify signature
    verification_key.verify(message, &signature_obj)
        .map_err(|_| QuantumCryptoError::SignatureVerificationFailed)?;
    
    Ok(())
}

/// ML-DSA signature state for protocol operations
pub struct MlDsaState {
    /// Our signing keypair
    pub keypair: Option<(MlDsaPublicKey, MlDsaPrivateKey)>,
    
    /// Cached signatures
    pub signatures: Vec<CachedSignature>,
}

/// Cached signature with metadata
#[derive(Debug, Clone)]
pub struct CachedSignature {
    pub message_hash: [u8; 32],
    pub signature: MlDsaSignature,
    pub timestamp: std::time::SystemTime,
}

impl MlDsaState {
    /// Create new ML-DSA state
    pub fn new() -> Self {
        Self {
            keypair: None,
            signatures: Vec::new(),
        }
    }
    
    /// Generate signing keypair
    pub fn generate_keypair(&mut self) -> Result<MlDsaPublicKey> {
        let (public_key, private_key) = generate_keypair()?;
        
        let public = MlDsaPublicKey(public_key);
        let private = MlDsaPrivateKey(private_key);
        
        self.keypair = Some((public.clone(), private));
        
        Ok(public)
    }
    
    /// Sign a message
    pub fn sign_message(&mut self, message: &[u8]) -> Result<MlDsaSignature> {
        let (_, private_key) = self.keypair.as_ref()
            .ok_or_else(|| QuantumCryptoError::InvalidKeyError(
                "No signing key available".to_string()
            ))?;
        
        let signature_bytes = sign(&private_key.0, message)?;
        let signature = MlDsaSignature(signature_bytes);
        
        // Cache signature
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(message);
        let message_hash = hasher.finalize().into();
        
        self.signatures.push(CachedSignature {
            message_hash,
            signature: signature.clone(),
            timestamp: std::time::SystemTime::now(),
        });
        
        // Keep cache size reasonable (last 100 signatures)
        if self.signatures.len() > 100 {
            self.signatures.drain(0..10);
        }
        
        Ok(signature)
    }
    
    /// Verify a signature
    pub fn verify_signature(
        &self,
        public_key: &MlDsaPublicKey,
        message: &[u8],
        signature: &MlDsaSignature,
    ) -> Result<()> {
        verify(&public_key.0, message, &signature.0)
    }
    
    /// Get cached signature if available
    pub fn get_cached_signature(&self, message: &[u8]) -> Option<&MlDsaSignature> {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(message);
        let message_hash: [u8; 32] = hasher.finalize().into();
        
        self.signatures
            .iter()
            .find(|s| s.message_hash == message_hash)
            .map(|s| &s.signature)
    }
}

/// Batch signature verification for efficiency
pub struct BatchVerifier {
    verifications: Vec<PendingVerification>,
}

struct PendingVerification {
    public_key: Vec<u8>,
    message: Vec<u8>,
    signature: Vec<u8>,
}

impl BatchVerifier {
    /// Create new batch verifier
    pub fn new() -> Self {
        Self {
            verifications: Vec::new(),
        }
    }
    
    /// Add signature to batch
    pub fn add(
        &mut self,
        public_key: &MlDsaPublicKey,
        message: &[u8],
        signature: &MlDsaSignature,
    ) {
        self.verifications.push(PendingVerification {
            public_key: public_key.0.clone(),
            message: message.to_vec(),
            signature: signature.0.clone(),
        });
    }
    
    /// Verify all signatures in batch
    pub fn verify_all(&self) -> Result<Vec<bool>> {
        let mut results = Vec::with_capacity(self.verifications.len());
        
        for verification in &self.verifications {
            let result = verify(
                &verification.public_key,
                &verification.message,
                &verification.signature,
            ).is_ok();
            
            results.push(result);
        }
        
        Ok(results)
    }
    
    /// Clear batch
    pub fn clear(&mut self) {
        self.verifications.clear();
    }
}

/// Aggregated signature for multiple signers (simplified)
pub struct AggregatedSignature {
    pub signatures: Vec<(ParticipantId, MlDsaSignature)>,
    pub message: Vec<u8>,
}

impl AggregatedSignature {
    /// Create new aggregated signature
    pub fn new(message: Vec<u8>) -> Self {
        Self {
            signatures: Vec::new(),
            message,
        }
    }
    
    /// Add a signature from a participant
    pub fn add_signature(&mut self, participant_id: ParticipantId, signature: MlDsaSignature) {
        self.signatures.push((participant_id, signature));
    }
    
    /// Verify all signatures
    pub fn verify_all(&self, participants: &[(ParticipantId, MlDsaPublicKey)]) -> Result<()> {
        for (participant_id, signature) in &self.signatures {
            // Find corresponding public key
            let public_key = participants
                .iter()
                .find(|(id, _)| id == participant_id)
                .map(|(_, key)| key)
                .ok_or_else(|| QuantumCryptoError::MlDsaError(
                    format!("No public key for participant {:?}", participant_id)
                ))?;
            
            // Verify signature
            verify(&public_key.0, &self.message, &signature.0)?;
        }
        
        Ok(())
    }
    
    /// Check if we have threshold signatures
    pub fn has_threshold(&self, threshold: usize) -> bool {
        self.signatures.len() >= threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ml_dsa_sign_verify() {
        let (public_key, private_key) = generate_keypair().unwrap();
        let message = b"Test message for ML-DSA";
        
        // Sign message
        let signature = sign(&private_key, message).unwrap();
        
        // Verify signature
        assert!(verify(&public_key, message, &signature).is_ok());
        
        // Verify with wrong message fails
        let wrong_message = b"Wrong message";
        assert!(verify(&public_key, wrong_message, &signature).is_err());
    }
    
    #[test]
    fn test_ml_dsa_state() {
        let mut state = MlDsaState::new();
        let public_key = state.generate_keypair().unwrap();
        
        let message = b"Test message";
        let signature = state.sign_message(message).unwrap();
        
        // Verify using state
        assert!(state.verify_signature(&public_key, message, &signature).is_ok());
        
        // Check cached signature
        assert!(state.get_cached_signature(message).is_some());
    }
    
    #[test]
    fn test_batch_verification() {
        let mut batch = BatchVerifier::new();
        
        // Generate multiple keypairs and signatures
        let messages = vec![
            b"Message 1".to_vec(),
            b"Message 2".to_vec(),
            b"Message 3".to_vec(),
        ];
        
        let mut keys_and_sigs = Vec::new();
        
        for message in &messages {
            let (public_key, private_key) = generate_keypair().unwrap();
            let signature = sign(&private_key, message).unwrap();
            
            keys_and_sigs.push((
                MlDsaPublicKey(public_key),
                MlDsaSignature(signature),
            ));
        }
        
        // Add to batch
        for (i, (public_key, signature)) in keys_and_sigs.iter().enumerate() {
            batch.add(public_key, &messages[i], signature);
        }
        
        // Verify all
        let results = batch.verify_all().unwrap();
        assert!(results.iter().all(|&r| r));
    }
    
    #[test]
    fn test_aggregated_signatures() {
        let message = b"Group message".to_vec();
        let mut agg_sig = AggregatedSignature::new(message.clone());
        
        // Create participants
        let mut participants = Vec::new();
        
        for i in 0..3 {
            let (public_key, private_key) = generate_keypair().unwrap();
            let participant_id = ParticipantId(i);
            
            // Sign message
            let signature = sign(&private_key, &message).unwrap();
            
            participants.push((participant_id.clone(), MlDsaPublicKey(public_key)));
            agg_sig.add_signature(participant_id, MlDsaSignature(signature));
        }
        
        // Verify all signatures
        assert!(agg_sig.verify_all(&participants).is_ok());
        
        // Check threshold
        assert!(agg_sig.has_threshold(2));
        assert!(agg_sig.has_threshold(3));
        assert!(!agg_sig.has_threshold(4));
    }
}