#!/bin/bash

echo "Fixing all enum variant mismatches..."

# Fix IdentityError::NotFound variants
find src -name "*.rs" -type f -exec sed -i '' 's/IdentityError::NotFound { id: [^}]* }/IdentityError::NotFound("Not found".to_string())/g' {} \;

# Fix StorageError::CorruptionDetected variants
find src -name "*.rs" -type f -exec sed -i '' 's/StorageError::CorruptionDetected { reason: \([^}]*\) }/StorageError::CorruptionDetected(\1)/g' {} \;

# Fix DhtError::InsufficientReplicas variants
find src -name "*.rs" -type f -exec sed -i '' 's/DhtError::InsufficientReplicas { available: [^,]*, required: [^}]* }/DhtError::InsufficientReplicas(0, 0)/g' {} \;

# Fix IdentityError::VerificationFailed variants
find src -name "*.rs" -type f -exec sed -i '' 's/IdentityError::VerificationFailed { reason: \([^}]*\) }/IdentityError::VerificationFailed(\1)/g' {} \;

# Fix IdentityError::InvalidFormat variants
find src -name "*.rs" -type f -exec sed -i '' 's/IdentityError::InvalidFormat { reason: \([^}]*\) }/IdentityError::InvalidFormat(\1)/g' {} \;

# Fix ConfigError::InvalidValue variants
find src -name "*.rs" -type f -exec sed -i '' 's/ConfigError::InvalidValue { value: \([^}]*\) }/ConfigError::InvalidValue(\1)/g' {} \;

# Fix StorageError::FileNotFound variants
find src -name "*.rs" -type f -exec sed -i '' 's/StorageError::FileNotFound { path: \([^}]*\) }/StorageError::FileNotFound(\1)/g' {} \;

# Fix P2pError to P2PError (case sensitivity)
find src -name "*.rs" -type f -exec sed -i '' 's/P2pError/P2PError/g' {} \;

echo "Done fixing enum variant mismatches"