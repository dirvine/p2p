#!/bin/bash

# Revert the underscore changes that are causing scope errors

# Fix content_type in learning.rs (it's used in function body)
sed -i '' 's/_content_type: ContentType,/content_type: ContentType,/' src/adaptive/learning.rs

# Fix content_hash in replication.rs line 197 - it IS used (kept underscore by mistake)
sed -i '' '197s/_content_hash: &ContentHash,/content_hash: \&ContentHash,/' src/adaptive/replication.rs

# Fix content_hash in replication.rs line 237 - it IS used in the function
sed -i '' '237s/_content_hash: &ContentHash,/content_hash: \&ContentHash,/' src/adaptive/replication.rs

# Keep _content as underscore on line 238 since it's not used
# Line 238 already has _content which is correct

# Fix line 393 - content_hash is NOT used in replicate function (keep underscore)
# sed -i '' '393s/_content_hash: &ContentHash,/content_hash: \&ContentHash,/' src/adaptive/replication.rs

# Fix stats assignment in q_learning_cache.rs by uncommenting it
sed -i '' 's/\/\/ stats = self\.cache_stats\.write/stats = self.cache_stats.write/' src/adaptive/q_learning_cache.rs

echo "Fixed scope errors"