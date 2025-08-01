#!/bin/bash

# Fix references to underscore-prefixed variables in learning.rs
sed -i '' 's/match content_type {/match _content_type {/' src/adaptive/learning.rs
sed -i '' 's/arm_idx = self.bandit_arm_for_content_type(content_type)/arm_idx = self.bandit_arm_for_content_type(_content_type)/' src/adaptive/learning.rs

# Fix references to underscore-prefixed variables in replication.rs  
sed -i '' 's/self.router.get_kademlia_nodes(content_hash, count + 10).await?/self.router.get_kademlia_nodes(_content_hash, count + 10).await?/' src/adaptive/replication.rs
sed -i '' 's/self.router.get_hyperbolic_nodes(content_hash, count + 10).await?/self.router.get_hyperbolic_nodes(_content_hash, count + 10).await?/' src/adaptive/replication.rs
sed -i '' 's/self.router.get_som_similar_nodes(content_hash, count + 10).await?/self.router.get_som_similar_nodes(_content_hash, count + 10).await?/' src/adaptive/replication.rs

# Fix references in storage.rs
sed -i '' 's/hash: &ContentHash/_hash: &ContentHash/' src/adaptive/storage.rs
sed -i '' 's/data: &\[u8\]/_data: &\[u8\]/' src/adaptive/storage.rs
sed -i '' 's/state: &DataState/_state: &DataState/' src/adaptive/storage.rs
sed -i '' 's/content: Vec<u8>/_content: Vec<u8>/' src/adaptive/replication.rs
sed -i '' 's/replicas: usize/_replicas: usize/' src/adaptive/replication.rs

echo "Fixed underscore variable references"