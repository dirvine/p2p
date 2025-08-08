#!/bin/bash

# Fix P2PError usage in identity_storage.rs

FILE="apps/saorsa/src-tauri/src/identity_storage.rs"

# Replace anyhow errors with proper types
# Serialization errors
sed -i '' 's/P2PError::Io(anyhow::anyhow!("Failed to serialize/P2PError::Serialization("Failed to serialize/g' "$FILE"
sed -i '' 's/P2PError::Io(anyhow::anyhow!("Failed to deserialize/P2PError::Serialization("Failed to deserialize/g' "$FILE"

# Decode errors (also serialization related)
sed -i '' 's/P2PError::Io(anyhow::anyhow!("Failed to decode/P2PError::Serialization("Failed to decode/g' "$FILE"
sed -i '' 's/P2PError::Io(anyhow::anyhow!("Failed to parse/P2PError::Serialization("Failed to parse/g' "$FILE"

# Other errors that should remain as Io but need different construction
sed -i '' 's/P2PError::Io(anyhow::anyhow!("Missing/P2PError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "Missing/g' "$FILE"
sed -i '' 's/P2PError::Io(anyhow::anyhow!("Invalid/P2PError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid/g' "$FILE"

# Fix the closing parentheses for Serialization errors (change from )) to .into())
perl -i -pe 's/P2PError::Serialization\("([^"]+): \{\}", e\)\)/P2PError::Serialization(format!("$1: {}", e).into())/g' "$FILE"
perl -i -pe 's/P2PError::Serialization\("([^"]+): \{\}",\s*e\s*\)\)/P2PError::Serialization(format!("$1: {}", e).into())/g' "$FILE"

echo "Fixed error handling in $FILE"