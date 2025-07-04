#!/bin/bash

# License header template
LICENSE_HEADER="// Copyright 2024 MaidSafe Limited
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

"

# Function to add license header to a file
add_license_header() {
    local file="$1"
    echo "Adding license header to: $file"
    
    # Create temporary file with license header + original content
    temp_file=$(mktemp)
    echo "$LICENSE_HEADER" > "$temp_file"
    cat "$file" >> "$temp_file"
    
    # Replace original file
    mv "$temp_file" "$file"
}

# Find all Rust files missing license headers
echo "Finding Rust files without license headers..."

for file in $(find . -name "*.rs" -not -path "./target/*" -not -path "./node_modules/*"); do
    if ! head -n 15 "$file" | grep -q "Copyright.*MaidSafe Limited"; then
        # Skip certain files that shouldn't have license headers
        case "$file" in
            */target/*|*/node_modules/*|*/.git/*)
                continue
                ;;
            ./test_*.rs|./create_*.rs|./simple_*.rs|./minimal_*.rs)
                # Skip temporary test/example files in root
                echo "Skipping temporary file: $file"
                continue
                ;;
            *)
                add_license_header "$file"
                ;;
        esac
    fi
done

echo "License headers added successfully!"