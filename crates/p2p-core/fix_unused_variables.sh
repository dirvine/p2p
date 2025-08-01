#!/bin/bash

# Fix unused variables by prefixing with underscore

# Fix learning.rs:717
sed -i '' 's/content_type: ContentType,/_content_type: ContentType,/' src/adaptive/learning.rs

# Fix replication.rs:197  
sed -i '' 's/content_hash: &ContentHash,/_content_hash: &ContentHash,/' src/adaptive/replication.rs

# Fix churn.rs:303
sed -i '' 's/if let Err(e) = handler/if let Err(_e) = handler/' src/adaptive/churn.rs

# Fix monitoring.rs:927
sed -i '' 's/for rule in rules {/for _rule in rules {/' src/adaptive/monitoring.rs

# Fix more unused variables from the errors
sed -i '' 's/for metric in metrics {/for _metric in metrics {/' src/adaptive/monitoring.rs
sed -i '' 's/for sample in samples {/for _sample in samples {/' src/adaptive/performance.rs
sed -i '' 's/pub async fn get_metadata(&self, content_hash: &ContentHash)/pub async fn get_metadata(&self, _content_hash: &ContentHash)/' src/adaptive/storage.rs
sed -i '' 's/state: &DataState,/_state: &DataState,/' src/adaptive/storage.rs
sed -i '' 's/hash: &ContentHash,/_hash: &ContentHash,/' src/adaptive/storage.rs
sed -i '' 's/content: Vec<u8>,/_content: Vec<u8>,/' src/adaptive/replication.rs
sed -i '' 's/replicas: usize,/_replicas: usize,/' src/adaptive/replication.rs
sed -i '' 's/data: &\[u8\],/_data: &\[u8\],/' src/adaptive/storage.rs
sed -i '' 's/async fn validate_chunk_list(&self, path: &Path)/async fn validate_chunk_list(&self, _path: &Path)/' src/adaptive/storage.rs

echo "Fixed unused variables warnings"