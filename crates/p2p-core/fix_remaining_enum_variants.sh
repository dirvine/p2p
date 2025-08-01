#!/bin/bash

# Fix StorageError::CorruptionDetected - should be tuple variant
sed -i '' 's/crate::error::StorageError::CorruptionDetected {/crate::error::StorageError::CorruptionDetected(/' src/adaptive/storage.rs
sed -i '' 's/reason: "Invalid chunk boundaries"[[:space:]]*}/\/\* reason: \*\/ "Invalid chunk boundaries".to_string())/' src/adaptive/storage.rs

# Fix StorageError::FileNotFound - should be tuple variant 
sed -i '' 's/crate::error::StorageError::FileNotFound { path }/crate::error::StorageError::FileNotFound(path)/' src/encrypted_key_storage.rs

# Fix IdentityError variants - should be tuple variants
sed -i '' 's/IdentityError::VerificationFailed { reason: msg }/IdentityError::VerificationFailed(msg)/' src/identity/node_identity.rs
sed -i '' 's/IdentityError::InvalidFormat { reason: "Invalid word" }/IdentityError::InvalidFormat("Invalid word".to_string())/' src/identity/node_identity.rs

# Fix DhtError::InsufficientReplicas - should be tuple variant
sed -i '' 's/crate::error::DhtError::InsufficientReplicas {[^}]*}/crate::error::DhtError::InsufficientReplicas("Insufficient replicas".to_string())/' src/dht_network_manager.rs

echo "Fixed remaining enum variant syntax errors"