#!/bin/bash

echo "Changing all copyright headers from 'MaidSafe Limited' to 'Saorsa Labs Limited'..."

# Find all Rust files and replace the copyright owner
find . -name "*.rs" -not -path "./target/*" -not -path "./node_modules/*" -exec sed -i.bak 's/Copyright 2024 MaidSafe Limited/Copyright 2024 Saorsa Labs Limited/g' {} \;

# Clean up backup files
find . -name "*.rs.bak" -not -path "./target/*" -not -path "./node_modules/*" -delete

echo "✅ Updated all copyright headers to use 'Saorsa Labs Limited'"

# Count files that now have the correct copyright
correct_count=$(find . -name "*.rs" -not -path "./target/*" -not -path "./node_modules/*" -exec grep -l "Copyright.*Saorsa Labs Limited" {} \; | wc -l)
echo "✅ Found $correct_count files with 'Saorsa Labs Limited' copyright"

# Check for any remaining MaidSafe references
remaining_maidsafe=$(find . -name "*.rs" -not -path "./target/*" -not -path "./node_modules/*" -exec grep -l "Copyright.*MaidSafe Limited" {} \; | wc -l)
if [ $remaining_maidsafe -gt 0 ]; then
    echo "⚠️  Warning: $remaining_maidsafe files still have 'MaidSafe Limited' copyright"
    find . -name "*.rs" -not -path "./target/*" -not -path "./node_modules/*" -exec grep -l "Copyright.*MaidSafe Limited" {} \;
else
    echo "✅ All MaidSafe copyright references have been updated"
fi