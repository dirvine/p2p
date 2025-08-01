#!/bin/bash

# Fix DhtError::InsufficientReplicas in skademlia.rs
sed -i '' 's/DhtError::InsufficientReplicas {/DhtError::InsufficientReplicas(/' src/dht/skademlia.rs
sed -i '' 's/available: initial_nodes.len(),/format!("Available: {}, Required: {}", initial_nodes.len(), self.path_count)/' src/dht/skademlia.rs
sed -i '' 's/required: self.path_count,//' src/dht/skademlia.rs
sed -i '' 's/})/))/' src/dht/skademlia.rs

# Fix IdentityError::VerificationFailed in node_identity.rs
sed -i '' 's/IdentityError::VerificationFailed {/IdentityError::VerificationFailed(/' src/identity/node_identity.rs
sed -i '' 's/reason: "Invalid proof of work".to_string()/"Invalid proof of work".to_string()/' src/identity/node_identity.rs
sed -i '' 's/}/))/' src/identity/node_identity.rs

# Fix IdentityError::InvalidFormat in four_words.rs
sed -i '' 's/IdentityError::InvalidFormat {/IdentityError::InvalidFormat(/' src/identity/four_words.rs
sed -i '' 's/reason: "Input must be at least 8 bytes for four-word address".to_string()/"Input must be at least 8 bytes for four-word address".to_string()/' src/identity/four_words.rs

# Fix IdentityError::InvalidFormat in identity_manager.rs
sed -i '' 's/IdentityError::InvalidFormat {/IdentityError::InvalidFormat(/' src/identity_manager.rs
sed -i '' 's/reason: "Invalid peer node ID".to_string()/"Invalid peer node ID".to_string()/' src/identity_manager.rs

echo "Fixed final enum variant errors"