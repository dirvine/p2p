#!/bin/bash

# Fix IdentityError::NotFound struct to tuple variant
sed -i '' 's/IdentityError::NotFound {[[:space:]]*id: "current"\.to_string()[[:space:]]*}/IdentityError::NotFound("current".to_string())/g' src/identity/manager.rs

# Fix StorageError::CorruptionDetected struct to tuple variant
sed -i '' 's/StorageError::CorruptionDetected {[[:space:]]*reason: "Snapshot checksum mismatch"\.to_string()[[:space:]]*}/StorageError::CorruptionDetected("Snapshot checksum mismatch".to_string())/g' src/persistent_state.rs

# Fix SecurityError::SignatureVerificationFailed to tuple variant with message
sed -i '' 's/SecurityError::SignatureVerificationFailed)/SecurityError::SignatureVerificationFailed("Signature verification failed".to_string()))/g' src/crypto_verify.rs
sed -i '' 's/SecurityError::SignatureVerificationFailed)/SecurityError::SignatureVerificationFailed("Signature verification failed".to_string()))/g' src/identity_manager.rs

# Fix IdentityError::InvalidFormat struct to tuple variant 
sed -i '' 's/IdentityError::InvalidFormat { reason: format!("invalid public key: {}", e) }/IdentityError::InvalidFormat(format!("invalid public key: {}", e))/g' src/identity_manager.rs

# Fix StorageError::FileNotFound struct to tuple variant
sed -i '' 's/StorageError::FileNotFound {[[:space:]]*path: "key_pair_cache"\.to_string()[[:space:]]*}/StorageError::FileNotFound("key_pair_cache".to_string())/g' src/identity_manager.rs

# Fix ConfigError::InvalidValue - remove the 'value' field which doesn't exist
sed -i '' '/ConfigError::InvalidValue {/,/}/s/value: metadata_size\.to_string(),//' src/identity_manager.rs

echo "Fixed enum variant syntax errors"