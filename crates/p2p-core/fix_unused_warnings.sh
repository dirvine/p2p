#!/bin/bash

# Fix unused imports in coordinator.rs
sed -i '' 's/StorageStrategy, NetworkChurnPrediction, ChurnStats,/StorageStrategy,/' src/adaptive/coordinator.rs

# Fix unused variables with underscore prefix
sed -i '' 's/async fn wait_for_response(&self, message_id: &str, peer_id: &PeerId)/async fn wait_for_response(\&self, message_id: \&str, _peer_id: \&PeerId)/' src/dht_network_manager.rs

# Fix unused variables in learning.rs  
sed -i '' 's/content_type: ContentType,/_content_type: ContentType,/' src/adaptive/learning.rs
sed -i '' 's/let state = self\.get_current_state_async/let _state = self.get_current_state_async/' src/adaptive/learning.rs

# Fix unused assignment in q_learning_cache.rs - comment out the reassignment
sed -i '' 's/stats = self\.cache_stats\.write/\/\/ stats = self.cache_stats.write/' src/adaptive/q_learning_cache.rs

# Fix unused variables in replication.rs
sed -i '' 's/content_hash: &ContentHash,/_content_hash: \&ContentHash,/' src/adaptive/replication.rs  
sed -i '' 's/content: &\[u8\],/_content: \&[u8],/' src/adaptive/replication.rs

# Fix unused variable multiplier in replication.rs
sed -i '' 's/pub async fn increase_global_replication(&self, multiplier: f64)/pub async fn increase_global_replication(\&self, _multiplier: f64)/' src/adaptive/replication.rs

echo "Fixed unused warnings"