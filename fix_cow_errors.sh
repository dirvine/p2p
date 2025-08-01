#!/bin/bash

# Fix format! calls that need to be converted to Cow<'static, str>

cd /Users/davidirvine/Desktop/Devel/projects/p2p

# Fix to_string() calls in error constructors
find crates -name "*.rs" -type f -exec sed -i '' 's/\(NetworkError::[A-Za-z]*(\)\([^)]*\)\.to_string()/\1\2.to_string().into()/g' {} +
find crates -name "*.rs" -type f -exec sed -i '' 's/\(DhtError::[A-Za-z]*(\)\([^)]*\)\.to_string()/\1\2.to_string().into()/g' {} +
find crates -name "*.rs" -type f -exec sed -i '' 's/\(IdentityError::[A-Za-z]*(\)\([^)]*\)\.to_string()/\1\2.to_string().into()/g' {} +
find crates -name "*.rs" -type f -exec sed -i '' 's/\(CryptoError::[A-Za-z]*(\)\([^)]*\)\.to_string()/\1\2.to_string().into()/g' {} +
find crates -name "*.rs" -type f -exec sed -i '' 's/\(StorageError::[A-Za-z]*(\)\([^)]*\)\.to_string()/\1\2.to_string().into()/g' {} +
find crates -name "*.rs" -type f -exec sed -i '' 's/\(TransportError::[A-Za-z]*(\)\([^)]*\)\.to_string()/\1\2.to_string().into()/g' {} +
find crates -name "*.rs" -type f -exec sed -i '' 's/\(McpError::[A-Za-z]*(\)\([^)]*\)\.to_string()/\1\2.to_string().into()/g' {} +

# Fix format! calls in error constructors
find crates -name "*.rs" -type f -exec sed -i '' 's/\(NetworkError::[A-Za-z]*(\)format!/\1format!/g' {} +
find crates -name "*.rs" -type f -exec sed -i '' 's/NetworkError::\([A-Za-z]*\)(format!(\([^)]*\)))/NetworkError::\1(format!(\2).into())/g' {} +
find crates -name "*.rs" -type f -exec sed -i '' 's/DhtError::\([A-Za-z]*\)(format!(\([^)]*\)))/DhtError::\1(format!(\2).into())/g' {} +
find crates -name "*.rs" -type f -exec sed -i '' 's/IdentityError::\([A-Za-z]*\)(format!(\([^)]*\)))/IdentityError::\1(format!(\2).into())/g' {} +
find crates -name "*.rs" -type f -exec sed -i '' 's/CryptoError::\([A-Za-z]*\)(format!(\([^)]*\)))/CryptoError::\1(format!(\2).into())/g' {} +
find crates -name "*.rs" -type f -exec sed -i '' 's/StorageError::\([A-Za-z]*\)(format!(\([^)]*\)))/StorageError::\1(format!(\2).into())/g' {} +
find crates -name "*.rs" -type f -exec sed -i '' 's/TransportError::\([A-Za-z]*\)(format!(\([^)]*\)))/TransportError::\1(format!(\2).into())/g' {} +
find crates -name "*.rs" -type f -exec sed -i '' 's/McpError::\([A-Za-z]*\)(format!(\([^)]*\)))/McpError::\1(format!(\2).into())/g' {} +
find crates -name "*.rs" -type f -exec sed -i '' 's/ConfigError::\([A-Za-z]*\)(format!(\([^)]*\)))/ConfigError::\1(format!(\2).into())/g' {} +
find crates -name "*.rs" -type f -exec sed -i '' 's/SecurityError::\([A-Za-z]*\)(format!(\([^)]*\)))/SecurityError::\1(format!(\2).into())/g' {} +
find crates -name "*.rs" -type f -exec sed -i '' 's/BootstrapError::\([A-Za-z]*\)(format!(\([^)]*\)))/BootstrapError::\1(format!(\2).into())/g' {} +

# Fix P2PError variant calls
find crates -name "*.rs" -type f -exec sed -i '' 's/P2PError::Serialization(format!(\([^)]*\)))/P2PError::Serialization(format!(\1).into())/g' {} +
find crates -name "*.rs" -type f -exec sed -i '' 's/P2PError::Validation(format!(\([^)]*\)))/P2PError::Validation(format!(\1).into())/g' {} +
find crates -name "*.rs" -type f -exec sed -i '' 's/P2PError::ResourceExhausted(format!(\([^)]*\)))/P2PError::ResourceExhausted(format!(\1).into())/g' {} +
find crates -name "*.rs" -type f -exec sed -i '' 's/P2PError::Internal(format!(\([^)]*\)))/P2PError::Internal(format!(\1).into())/g' {} +
find crates -name "*.rs" -type f -exec sed -i '' 's/P2PError::Encoding(format!(\([^)]*\)))/P2PError::Encoding(format!(\1).into())/g' {} +

echo "Fixed Cow<'static, str> conversions"