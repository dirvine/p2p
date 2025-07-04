#!/bin/bash

# License header to add
LICENSE_HEADER='// Copyright 2024 MaidSafe Limited
//
// This software is dual-licensed under:
// - GNU Affero General Public License v3.0 or later (AGPL-3.0-or-later)
// - Commercial License
//
// For AGPL-3.0 license, see LICENSE-AGPL-3.0
// For commercial licensing, contact: saorsalabs@gmail.com
//
// Unless required by applicable law or agreed to in writing, software
// distributed under these licenses is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.

'

echo "Finding all Rust files missing license headers..."

# Find all Rust files that don't have the license header
missing_files=$(find . -name "*.rs" -not -path "./target/*" -not -path "./node_modules/*" -exec sh -c 'head -n 15 "$1" | grep -q "Copyright.*MaidSafe Limited" || echo "$1"' _ {} \;)

if [ -z "$missing_files" ]; then
    echo "✅ All Rust files already have license headers"
    exit 0
fi

echo "Found $(echo "$missing_files" | wc -l) files missing license headers"

# Add license header to each file
for file in $missing_files; do
    echo "Adding license header to: $file"
    
    # Create a temporary file with the license header and original content
    temp_file=$(mktemp)
    echo "$LICENSE_HEADER" > "$temp_file"
    cat "$file" >> "$temp_file"
    
    # Replace the original file
    mv "$temp_file" "$file"
done

echo "✅ License headers added to all files"

# Clean up any empty lines at the beginning of files
echo "Cleaning up formatting..."
for file in $missing_files; do
    # Remove empty lines at the start, but preserve the license header
    sed -i.bak '/^$/N;/^\n$/d' "$file" 2>/dev/null || true
    rm -f "${file}.bak" 2>/dev/null || true
done

echo "✅ All license headers fixed!"